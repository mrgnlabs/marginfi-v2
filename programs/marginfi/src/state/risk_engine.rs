use super::{
    bank::Bank,
    marginfi_account::{BalanceSide, BankAccountWithPriceFeed, MarginfiAccount, RequirementType},
    marginfi_group::RiskTier,
};
use crate::{
    check,
    constants::{BANKRUPT_THRESHOLD, ZERO_AMOUNT_THRESHOLD},
    debug, math_error,
    prelude::{MarginfiError, MarginfiResult},
};
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use std::ops::Not;

pub const IN_FLASHLOAN_FLAG: u64 = 1 << 1;

pub enum RiskRequirementType {
    Initial,
    Maintenance,
    Equity,
}

impl RiskRequirementType {
    pub fn to_weight_type(&self) -> RequirementType {
        match self {
            RiskRequirementType::Initial => RequirementType::Initial,
            RiskRequirementType::Maintenance => RequirementType::Maintenance,
            RiskRequirementType::Equity => RequirementType::Equity,
        }
    }
}
pub struct RiskEngine<'a, 'info> {
    marginfi_account: &'a MarginfiAccount,
    bank_accounts_with_price: Vec<BankAccountWithPriceFeed<'a, 'info>>,
}

impl<'info> RiskEngine<'_, 'info> {
    pub fn new<'a>(
        marginfi_account: &'a MarginfiAccount,
        remaining_ais: &'info [AccountInfo<'info>],
    ) -> MarginfiResult<RiskEngine<'a, 'info>> {
        check!(
            !marginfi_account.get_flag(IN_FLASHLOAN_FLAG),
            MarginfiError::AccountInFlashloan
        );

        Self::new_no_flashloan_check(marginfi_account, remaining_ais)
    }

    /// Internal constructor used either after manually checking account is not in a flashloan,
    /// or explicity checking health for flashloan enabled actions.
    fn new_no_flashloan_check<'a>(
        marginfi_account: &'a MarginfiAccount,
        remaining_ais: &'info [AccountInfo<'info>],
    ) -> MarginfiResult<RiskEngine<'a, 'info>> {
        let bank_accounts_with_price =
            BankAccountWithPriceFeed::load(&marginfi_account.lending_account, remaining_ais)?;

        Ok(RiskEngine {
            marginfi_account,
            bank_accounts_with_price,
        })
    }

    /// Checks account is healthy after performing actions that increase risk (removing liquidity).
    ///
    /// `IN_FLASHLOAN_FLAG` behavior.
    /// - Health check is skipped.
    /// - `remaining_ais` can be an empty vec.
    pub fn check_account_init_health<'a>(
        marginfi_account: &'a MarginfiAccount,
        remaining_ais: &'info [AccountInfo<'info>],
    ) -> MarginfiResult<()> {
        if marginfi_account.get_flag(IN_FLASHLOAN_FLAG) {
            return Ok(());
        }

        Self::new_no_flashloan_check(marginfi_account, remaining_ais)?
            .check_account_health(RiskRequirementType::Initial)?;

        Ok(())
    }

    /// Returns the total assets and liabilities of the account in the form of (assets, liabilities)
    pub fn get_account_health_components(
        &self,
        requirement_type: RiskRequirementType,
    ) -> MarginfiResult<(I80F48, I80F48)> {
        let mut total_assets = I80F48::ZERO;
        let mut total_liabilities = I80F48::ZERO;

        for a in &self.bank_accounts_with_price {
            let (assets, liabilities) =
                a.calc_weighted_assets_and_liabilities_values(requirement_type.to_weight_type())?;

            debug!(
                "Balance {}, assets: {}, liabilities: {}",
                a.balance.bank_pk, assets, liabilities
            );

            total_assets = total_assets.checked_add(assets).ok_or_else(math_error!())?;
            total_liabilities = total_liabilities
                .checked_add(liabilities)
                .ok_or_else(math_error!())?;
        }

        Ok((total_assets, total_liabilities))
    }

    pub fn get_account_health(
        &'info self,
        requirement_type: RiskRequirementType,
    ) -> MarginfiResult<I80F48> {
        let (total_weighted_assets, total_weighted_liabilities) =
            self.get_account_health_components(requirement_type)?;

        Ok(total_weighted_assets
            .checked_sub(total_weighted_liabilities)
            .ok_or_else(math_error!())?)
    }

    fn check_account_health(&self, requirement_type: RiskRequirementType) -> MarginfiResult {
        let (total_weighted_assets, total_weighted_liabilities) =
            self.get_account_health_components(requirement_type)?;

        debug!(
            "check_health: assets {} - liabs: {}",
            total_weighted_assets, total_weighted_liabilities
        );

        check!(
            total_weighted_assets >= total_weighted_liabilities,
            MarginfiError::RiskEngineInitRejected
        );

        self.check_account_risk_tiers()?;

        Ok(())
    }

    /// Checks
    /// 1. Account is liquidatable
    /// 2. Account has an outstanding liability for the provided liability bank
    pub fn check_pre_liquidation_condition_and_get_account_health(
        &self,
        bank_pk: &Pubkey,
    ) -> MarginfiResult<I80F48> {
        check!(
            !self.marginfi_account.get_flag(IN_FLASHLOAN_FLAG),
            MarginfiError::AccountInFlashloan
        );

        let liability_bank_balance = self
            .bank_accounts_with_price
            .iter()
            .find(|a| a.balance.bank_pk == *bank_pk)
            .ok_or(MarginfiError::LendingAccountBalanceNotFound)?;

        check!(
            liability_bank_balance
                .is_empty(BalanceSide::Liabilities)
                .not(),
            MarginfiError::IllegalLiquidation
        );

        check!(
            liability_bank_balance.is_empty(BalanceSide::Assets),
            MarginfiError::IllegalLiquidation
        );

        let (assets, liabs) =
            self.get_account_health_components(RiskRequirementType::Maintenance)?;

        let account_health = assets.checked_sub(liabs).ok_or_else(math_error!())?;

        debug!(
            "pre_liquidation_health: {} ({} - {})",
            account_health, assets, liabs
        );

        check!(
            account_health <= I80F48::ZERO,
            MarginfiError::IllegalLiquidation,
            "Account not unhealthy"
        );

        Ok(account_health)
    }

    /// Check that the account is at most at the maintenance requirement level post liquidation.
    /// This check is used to ensure two things in the liquidation process:
    /// 1. We check that the liquidatee's remaining liability is not empty
    /// 2. Liquidatee account was below the maintenance requirement level before liquidation (as health can only increase, because liquidations always pay down liabilities)
    /// 3. Liquidator didn't liquidate too many assets that would result in unnecessary loss for the liquidatee.
    ///
    /// This check works on the assumption that the liquidation always results in a reduction of risk.
    ///
    /// 1. We check that the paid off liability is not zero. Assuming the liquidation always pays off some liability, this ensures that the liquidation was not too large.
    /// 2. We check that the account is still at most at the maintenance requirement level. This ensures that the liquidation was not too large overall.
    pub fn check_post_liquidation_condition_and_get_account_health(
        &self,
        bank_pk: &Pubkey,
        pre_liquidation_health: I80F48,
    ) -> MarginfiResult<I80F48> {
        check!(
            !self.marginfi_account.get_flag(IN_FLASHLOAN_FLAG),
            MarginfiError::AccountInFlashloan
        );

        let liability_bank_balance = self
            .bank_accounts_with_price
            .iter()
            .find(|a| a.balance.bank_pk == *bank_pk)
            .unwrap();

        check!(
            liability_bank_balance
                .is_empty(BalanceSide::Liabilities)
                .not(),
            MarginfiError::IllegalLiquidation,
            "Liability payoff too severe, exhausted liability"
        );

        check!(
            liability_bank_balance.is_empty(BalanceSide::Assets),
            MarginfiError::IllegalLiquidation,
            "Liability payoff too severe, liability balance has assets"
        );

        let (assets, liabs) =
            self.get_account_health_components(RiskRequirementType::Maintenance)?;

        let account_health = assets.checked_sub(liabs).ok_or_else(math_error!())?;

        check!(
            account_health <= I80F48::ZERO,
            MarginfiError::IllegalLiquidation,
            "Liquidation too severe, account above maintenance requirement"
        );

        debug!(
            "account_health: {} ({} - {}), pre_liquidation_health: {}",
            account_health, assets, liabs, pre_liquidation_health,
        );

        check!(
            account_health > pre_liquidation_health,
            MarginfiError::IllegalLiquidation,
            "Post liquidation health worse"
        );

        Ok(account_health)
    }

    /// Check that the account is in a bankrupt state.
    /// Account needs to be insolvent and total value of assets need to be below the bankruptcy threshold.
    pub fn check_account_bankrupt(&self) -> MarginfiResult {
        let (total_assets, total_liabilities) =
            self.get_account_health_components(RiskRequirementType::Equity)?;

        check!(
            !self.marginfi_account.get_flag(IN_FLASHLOAN_FLAG),
            MarginfiError::AccountInFlashloan
        );

        msg!(
            "check_bankrupt: assets {} - liabs: {}",
            total_assets,
            total_liabilities
        );

        check!(
            total_assets < total_liabilities,
            MarginfiError::AccountNotBankrupt
        );
        check!(
            total_assets < BANKRUPT_THRESHOLD && total_liabilities > ZERO_AMOUNT_THRESHOLD,
            MarginfiError::AccountNotBankrupt
        );

        Ok(())
    }

    fn check_account_risk_tiers<'a>(&'a self) -> MarginfiResult
    where
        'info: 'a,
    {
        let balances_with_liablities = self
            .bank_accounts_with_price
            .iter()
            .filter(|a| a.balance.is_empty(BalanceSide::Liabilities).not());

        let n_balances_with_liablities = balances_with_liablities.clone().count();

        let is_in_isolated_risk_tier = balances_with_liablities.clone().any(|a| {
            // SAFETY: We are shortening 'info -> 'a
            let shorter_bank: &'a AccountInfo<'a> = unsafe { core::mem::transmute(&a.bank) };
            AccountLoader::<Bank>::try_from(shorter_bank)
                .unwrap()
                .load()
                .unwrap()
                .config
                .risk_tier
                == RiskTier::Isolated
        });

        check!(
            !is_in_isolated_risk_tier || n_balances_with_liablities == 1,
            MarginfiError::IsolatedAccountIllegalState
        );

        Ok(())
    }
}
