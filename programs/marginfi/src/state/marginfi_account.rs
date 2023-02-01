use super::marginfi_group::{Bank, WrappedI80F48};
use crate::{
    check,
    constants::{CONF_INTERVAL_MULTIPLE, EMPTY_BALANCE_THRESHOLD, MAX_PRICE_AGE_SEC},
    math_error,
    prelude::{MarginfiError, MarginfiResult},
};
use anchor_lang::prelude::*;
use anchor_spl::token::Transfer;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use pyth_sdk_solana::PriceFeed;
use std::{
    cmp::{max, min},
    ops::Not,
};
#[cfg(any(feature = "test", feature = "client"))]
use type_layout::TypeLayout;

#[account(zero_copy)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
pub struct MarginfiAccount {
    pub group: Pubkey,
    pub authority: Pubkey,
    pub lending_account: LendingAccount,
}

impl MarginfiAccount {
    /// Set the initial data for the marginfi account.
    pub fn initialize(&mut self, group: Pubkey, authority: Pubkey) {
        self.authority = authority;
        self.group = group;
    }

    pub fn get_remaining_accounts_len(&self) -> usize {
        self.lending_account
            .balances
            .iter()
            .filter(|b| b.active)
            .count()
            * 2 // TODO: Make account count oracle setup specific
    }
}

const EXP_10_I80F48: [I80F48; 15] = [
    I80F48!(1),
    I80F48!(10),
    I80F48!(100),
    I80F48!(1_000),
    I80F48!(10_000),
    I80F48!(100_000),
    I80F48!(1_000_000),
    I80F48!(10_000_000),
    I80F48!(100_000_000),
    I80F48!(1_000_000_000),
    I80F48!(10_000_000_000),
    I80F48!(100_000_000_000),
    I80F48!(1_000_000_000_000),
    I80F48!(10_000_000_000_000),
    I80F48!(100_000_000_000_000),
];

#[inline(always)]
fn pyth_price_components_to_i80f48(price: I80F48, exponent: i32) -> MarginfiResult<I80F48> {
    let scaling_factor = EXP_10_I80F48[exponent.unsigned_abs() as usize];

    let price = if exponent == 0 {
        price
    } else if exponent < 0 {
        price
            .checked_div(scaling_factor)
            .ok_or_else(math_error!())?
    } else {
        price
            .checked_mul(scaling_factor)
            .ok_or_else(math_error!())?
    };

    Ok(price)
}

#[derive(Debug)]
pub enum BalanceIncreaseType {
    Any,
    RepayOnly,
    DepositOnly,
}

#[derive(Debug)]
pub enum BalanceDecreaseType {
    Any,
    WithdrawOnly,
    BorrowOnly,
}

pub enum WeightType {
    Initial,
    Maintenance,
}

pub struct BankAccountWithPriceFeed<'a> {
    bank: Bank,
    price_feed: PriceFeed,
    balance: &'a Balance,
}

pub enum BalanceSide {
    Assets,
    Liabilities,
}

impl<'a> BankAccountWithPriceFeed<'a> {
    pub fn load<'b: 'a, 'info: 'a + 'b>(
        lending_account: &'a LendingAccount,
        remaining_ais: &'b [AccountInfo<'info>],
    ) -> MarginfiResult<Vec<BankAccountWithPriceFeed<'a>>> {
        msg!(
            "Expecting {} remaining accounts",
            lending_account.get_active_balances_iter().count() * 2
        );
        msg!("Got {} remaining accounts", remaining_ais.len());

        check!(
            lending_account.get_active_balances_iter().count() * 2 == remaining_ais.len(),
            MarginfiError::MissingPythOrBankAccount
        );

        lending_account
            .get_active_balances_iter()
            .enumerate()
            .map(|(i, b)| {
                let bank_index = i * 2;
                let pyth_index = bank_index + 1;

                let bank_ai = remaining_ais.get(bank_index).unwrap();

                check!(b.bank_pk.eq(bank_ai.key), MarginfiError::InvalidBankAccount);
                let pyth_ai = remaining_ais.get(pyth_index).unwrap();

                let bank_al = AccountLoader::<Bank>::try_from(bank_ai)?;
                let bank = bank_al.load()?;

                let price_feed = bank.load_price_feed_from_account_info(pyth_ai)?;

                Ok(BankAccountWithPriceFeed {
                    bank: *bank,
                    price_feed,
                    balance: b,
                })
            })
            .collect::<Result<Vec<_>>>()
    }

    #[inline(always)]
    pub fn calc_weighted_assets_and_liabilities_values(
        &self,
        weight_type: WeightType,
    ) -> MarginfiResult<(I80F48, I80F48)> {
        let (worst_price, best_price) = get_price_range(&self.price_feed)?;

        let asset_amount = self
            .bank
            .get_asset_amount(self.balance.asset_shares.into())?;
        let liability_amount = self
            .bank
            .get_liability_amount(self.balance.liability_shares.into())?;
        let (asset_weight, liability_weight) = self.bank.config.get_weights(weight_type);

        let mint_decimals = self.bank.mint_decimals;

        Ok((
            calc_asset_value(asset_amount, worst_price, mint_decimals, Some(asset_weight))?,
            calc_asset_value(
                liability_amount,
                best_price,
                mint_decimals,
                Some(liability_weight),
            )?,
        ))
    }

    #[inline]
    pub fn is_empty(&self, side: BalanceSide) -> bool {
        self.balance.is_empty(side)
    }
}

/// Get a normalized price range for the given price feed.
/// The range is the price +/- the CONF_INTERVAL_MULTIPLE * confidence interval.
pub fn get_price_range(pf: &PriceFeed) -> MarginfiResult<(I80F48, I80F48)> {
    let price_state = pf
        .get_ema_price_no_older_than(Clock::get()?.unix_timestamp, MAX_PRICE_AGE_SEC)
        .ok_or(MarginfiError::StaleOracle)?;

    let base_price =
        pyth_price_components_to_i80f48(I80F48::from_num(price_state.price), price_state.expo)?;

    let price_range =
        pyth_price_components_to_i80f48(I80F48::from_num(price_state.conf), price_state.expo)?
            .checked_mul(CONF_INTERVAL_MULTIPLE)
            .ok_or_else(math_error!())?;

    let lowest_price = base_price
        .checked_sub(price_range)
        .ok_or_else(math_error!())?;
    let highest_price = base_price
        .checked_add(price_range)
        .ok_or_else(math_error!())?;

    Ok((lowest_price, highest_price))
}

pub fn get_price(pf: &PriceFeed) -> MarginfiResult<I80F48> {
    let price_state = pf
        .get_ema_price_no_older_than(Clock::get()?.unix_timestamp, MAX_PRICE_AGE_SEC)
        .ok_or(MarginfiError::StaleOracle)?;

    pyth_price_components_to_i80f48(I80F48::from_num(price_state.price), price_state.expo)
}

/// Calculate the value of an asset, given its quantity with a decimal exponent, and a price with a decimal exponent, and an optional weight.
#[inline]
pub fn calc_asset_value(
    asset_amount: I80F48,
    price: I80F48,
    mint_decimals: u8,
    weight: Option<I80F48>,
) -> MarginfiResult<I80F48> {
    if asset_amount == I80F48::ZERO {
        return Ok(I80F48::ZERO);
    }

    let scaling_factor = EXP_10_I80F48[mint_decimals as usize];

    let weighted_asset_amount = if let Some(weight) = weight {
        asset_amount.checked_mul(weight).unwrap()
    } else {
        asset_amount
    };

    msg!(
        "weighted_asset_qt: {}, price: {}, expo: {}",
        weighted_asset_amount,
        price,
        mint_decimals
    );

    let asset_value = weighted_asset_amount
        .checked_mul(price)
        .ok_or_else(math_error!())?
        .checked_div(scaling_factor)
        .ok_or_else(math_error!())?;

    Ok(asset_value)
}

#[inline]
pub fn calc_asset_quantity(
    asset_value: I80F48,
    price: I80F48,
    mint_decimals: u8,
) -> MarginfiResult<I80F48> {
    let scaling_factor = EXP_10_I80F48[mint_decimals as usize];

    let asset_qt = asset_value
        .checked_mul(scaling_factor)
        .ok_or_else(math_error!())?
        .checked_div(price)
        .ok_or_else(math_error!())?;

    Ok(asset_qt)
}

pub enum RiskRequirementType {
    Initial,
    Maintenance,
}

impl RiskRequirementType {
    pub fn to_weight_type(&self) -> WeightType {
        match self {
            RiskRequirementType::Initial => WeightType::Initial,
            RiskRequirementType::Maintenance => WeightType::Maintenance,
        }
    }
}

pub struct RiskEngine<'a> {
    bank_accounts_with_price: Vec<BankAccountWithPriceFeed<'a>>,
}

impl<'a> RiskEngine<'a> {
    pub fn new<'b: 'a, 'info: 'a + 'b>(
        marginfi_account: &'a MarginfiAccount,
        remaining_ais: &'b [AccountInfo<'info>],
    ) -> MarginfiResult<Self> {
        let bank_accounts_with_price =
            BankAccountWithPriceFeed::load(&marginfi_account.lending_account, remaining_ais)?;

        Ok(Self {
            bank_accounts_with_price,
        })
    }

    /// Returns the total assets and liabilities of the account in the form of (assets, liabilities)
    pub fn get_account_health_components(
        &self,
        requirement_type: RiskRequirementType,
    ) -> MarginfiResult<(I80F48, I80F48)> {
        Ok(self
            .bank_accounts_with_price
            .iter()
            .map(|a| {
                a.calc_weighted_assets_and_liabilities_values(requirement_type.to_weight_type())
            })
            .try_fold(
                (I80F48::ZERO, I80F48::ZERO),
                |(total_assets, total_liabilities), res| {
                    let (assets, liabilities) = res?;
                    let total_assets_sum =
                        total_assets.checked_add(assets).ok_or_else(math_error!())?;
                    let total_liabilities_sum = total_liabilities
                        .checked_add(liabilities)
                        .ok_or_else(math_error!())?;

                    Ok::<_, ProgramError>((total_assets_sum, total_liabilities_sum))
                },
            )?)
    }

    pub fn get_account_health(
        &self,
        requirement_type: RiskRequirementType,
    ) -> MarginfiResult<I80F48> {
        let (total_weighted_assets, total_weighted_liabilities) =
            self.get_account_health_components(requirement_type)?;

        Ok(total_weighted_assets
            .checked_sub(total_weighted_liabilities)
            .ok_or_else(math_error!())?)
    }

    pub fn check_account_health(&self, requirement_type: RiskRequirementType) -> MarginfiResult {
        let (total_weighted_assets, total_weighted_liabilities) =
            self.get_account_health_components(requirement_type)?;

        msg!(
            "check_health: assets {} - liabs: {}",
            total_weighted_assets,
            total_weighted_liabilities
        );

        check!(
            total_weighted_assets >= total_weighted_liabilities,
            MarginfiError::BadAccountHealth
        );

        Ok(())
    }

    /// Checks
    /// 1. Account is liquidatable
    /// 2. Account has an outstanding liability for the provided liablity bank
    pub fn check_pre_liquidation_condition_and_get_account_health(
        &self,
        bank_pk: &Pubkey,
    ) -> MarginfiResult<I80F48> {
        let liability_bank_balance = self
            .bank_accounts_with_price
            .iter()
            .find(|a| a.balance.bank_pk == *bank_pk)
            .unwrap();

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

        let account_health = self.get_account_health(RiskRequirementType::Maintenance)?;

        check!(
            account_health <= I80F48::ZERO,
            MarginfiError::IllegalLiquidation
        );

        Ok(account_health)
    }

    /// Check that the account is at most at the maintenance requirement level post liquidation.
    /// This check is used to ensure two things in the liquidation process:
    /// 1. Liquidatee account was below the maintenance requirement level before liquidation (as health can only increase, because liquidations always pay down liabilities)
    /// 2. Liquidator didn't liquidate too many assets that would result in unnecessary loss for the liquidatee.
    ///
    /// This check works on the assumption that the liquidation always results in a reduction of risk.
    ///
    /// 1. We check that the paid off liability is not zero. Assuming the liquidation always pays off some liability, this ensures that the liquidation was not too large.
    /// 2. We check that the account is still at most at the maintenance requirement level. This ensures that the liquidation was not too large overall.
    pub fn check_post_liquidation_account_health(
        &self,
        bank_pk: &Pubkey,
        pre_liquidation_health: I80F48,
    ) -> MarginfiResult {
        let liability_bank_balance = self
            .bank_accounts_with_price
            .iter()
            .find(|a| a.balance.bank_pk == *bank_pk)
            .unwrap();

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

        let account_health = self.get_account_health(RiskRequirementType::Maintenance)?;

        check!(
            account_health <= I80F48::ZERO,
            MarginfiError::IllegalLiquidation
        );

        msg!(
            "account_health: {}, pre_liquidation_health: {}",
            account_health,
            pre_liquidation_health
        );

        check!(
            account_health > pre_liquidation_health,
            MarginfiError::IllegalLiquidation
        );

        Ok(())
    }

    /// Check that the account is in a bankrupt state.
    pub fn check_account_bankrupt(&self) -> MarginfiResult {
        let (total_weighted_assets, total_weighted_liabilities) =
            self.get_account_health_components(RiskRequirementType::Initial)?;

        msg!(
            "check_bankrupt: assets {} - liabs: {}",
            total_weighted_assets,
            total_weighted_liabilities
        );

        check!(
            total_weighted_assets == I80F48::ZERO && total_weighted_liabilities > I80F48::ZERO,
            MarginfiError::AccountNotBankrupt
        );

        Ok(())
    }
}

const MAX_LENDING_ACCOUNT_BALANCES: usize = 16;

#[zero_copy]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
pub struct LendingAccount {
    pub balances: [Balance; MAX_LENDING_ACCOUNT_BALANCES],
}

impl LendingAccount {
    #[cfg(any(feature = "test", feature = "client"))]
    pub fn get_balance(&self, bank_pk: &Pubkey) -> Option<&Balance> {
        self.balances
            .iter()
            .find(|balance| balance.active && balance.bank_pk.eq(bank_pk))
    }

    pub fn get_first_empty_balance(&self) -> Option<usize> {
        self.balances.iter().position(|b| !b.active)
    }

    pub fn get_active_balances_iter(&self) -> impl Iterator<Item = &Balance> {
        self.balances.iter().filter(|b| b.active)
    }

    pub fn get_active_balances_iter_mut(&mut self) -> impl Iterator<Item = &mut Balance> {
        self.balances.iter_mut().filter(|b| b.active)
    }
}

#[zero_copy]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
pub struct Balance {
    pub active: bool,
    pub bank_pk: Pubkey,
    pub asset_shares: WrappedI80F48,
    pub liability_shares: WrappedI80F48,
}

impl Balance {
    /// Check whether a balance is empty while accounting for any rounding errors
    /// that might have occured during depositing/withdrawing.
    #[inline]
    pub fn is_empty(&self, side: BalanceSide) -> bool {
        let shares: I80F48 = match side {
            BalanceSide::Assets => self.asset_shares,
            BalanceSide::Liabilities => self.liability_shares,
        }
        .into();

        shares < EMPTY_BALANCE_THRESHOLD
    }

    pub fn change_asset_shares(&mut self, delta: I80F48) -> MarginfiResult {
        let asset_shares: I80F48 = self.asset_shares.into();
        self.asset_shares = asset_shares
            .checked_add(delta)
            .ok_or_else(math_error!())?
            .into();
        Ok(())
    }

    pub fn change_liability_shares(&mut self, delta: I80F48) -> MarginfiResult {
        let liability_shares: I80F48 = self.liability_shares.into();
        self.liability_shares = liability_shares
            .checked_add(delta)
            .ok_or_else(math_error!())?
            .into();
        Ok(())
    }

    pub fn close(&mut self) {
        self.active = false;
        self.asset_shares = I80F48::ZERO.into();
        self.liability_shares = I80F48::ZERO.into();
        self.bank_pk = Pubkey::default();
    }
}

pub struct BankAccountWrapper<'a> {
    pub balance: &'a mut Balance,
    bank: &'a mut Bank,
}

impl<'a> BankAccountWrapper<'a> {
    // Find existing user lending account balance by bank address.
    pub fn find<'b>(
        bank_pk: &Pubkey,
        bank: &'a mut Bank,
        lending_account: &'a mut LendingAccount,
    ) -> MarginfiResult<BankAccountWrapper<'a>> {
        let balance = lending_account
            .get_active_balances_iter_mut()
            .find(|balance| balance.bank_pk.eq(bank_pk))
            .ok_or_else(|| error!(MarginfiError::BankAccoutNotFound))?;

        Ok(Self { balance, bank })
    }

    // Find existing user lending account balance by bank address.
    // Create it if not found.
    pub fn find_or_create<'b>(
        bank_pk: &Pubkey,
        bank: &'a mut Bank,
        lending_account: &'a mut LendingAccount,
    ) -> MarginfiResult<BankAccountWrapper<'a>> {
        let balance_index = lending_account
            .get_active_balances_iter()
            .position(|balance| balance.bank_pk.eq(bank_pk));

        match balance_index {
            Some(_) => {
                let balance = lending_account
                    .get_active_balances_iter_mut()
                    .find(|balance| balance.bank_pk.eq(bank_pk))
                    .ok_or_else(|| error!(MarginfiError::BankAccoutNotFound))?;

                Ok(Self { balance, bank })
            }
            None => {
                let empty_index = lending_account
                    .get_first_empty_balance()
                    .ok_or_else(|| error!(MarginfiError::LendingAccountBalanceSlotsFull))?;

                lending_account.balances[empty_index] = Balance {
                    active: true,
                    bank_pk: *bank_pk,
                    asset_shares: I80F48::ZERO.into(),
                    liability_shares: I80F48::ZERO.into(),
                };

                Ok(Self {
                    balance: lending_account.balances.get_mut(empty_index).unwrap(),
                    bank,
                })
            }
        }
    }

    // ------------ Borrow / Lend primitives

    /// Deposit an asset, will error if there is an existing liability - repaying is not allowed.
    pub fn deposit(&mut self, amount: I80F48) -> MarginfiResult {
        self.increase_balance_internal(amount, BalanceIncreaseType::DepositOnly)
    }

    /// Withdraw an asset, will error if there is not enough asset - borrowing is not allowed.
    pub fn withdraw(&mut self, amount: I80F48) -> MarginfiResult {
        self.decrease_balance_internal(amount, BalanceDecreaseType::WithdrawOnly)
    }

    /// Incur a borrow, will error if there is an existing asset - withdrawing is not allowed.
    pub fn borrow(&mut self, amount: I80F48) -> MarginfiResult {
        self.decrease_balance_internal(amount, BalanceDecreaseType::BorrowOnly)
    }

    /// Repay a liability, will error if there is not enough liability - depositing is not allowed.
    pub fn repay(&mut self, amount: I80F48) -> MarginfiResult {
        self.increase_balance_internal(amount, BalanceIncreaseType::RepayOnly)
    }

    // ------------ Hybrid operations for seamless repay + deposit / withdraw + borrow

    /// Repay liability and deposit/increase asset depending on
    /// the specified deposit amount and the existing balance.
    pub fn increase_balance(&mut self, amount: I80F48) -> MarginfiResult {
        self.increase_balance_internal(amount, BalanceIncreaseType::Any)
    }

    /// Withdraw asset and create/increase liability depending on
    /// the specified deposit amount and the existing balance.
    pub fn decrease_balance(&mut self, amount: I80F48) -> MarginfiResult {
        self.decrease_balance_internal(amount, BalanceDecreaseType::Any)
    }

    /// Withdraw existing asset in full - will error if there is no asset.
    pub fn withdraw_all(&mut self) -> MarginfiResult<u64> {
        let balance = &mut self.balance;
        let bank = &mut self.bank;

        let current_asset_amount = bank.get_asset_amount(balance.asset_shares.into())?;

        msg!(
            "Withdrawing all: {} of {} in {}",
            current_asset_amount,
            bank.mint,
            balance.bank_pk,
        );

        check!(
            current_asset_amount.is_positive(),
            MarginfiError::NoAssetFound
        );

        balance.close();
        let asset_shares_decrease = bank.get_asset_shares(current_asset_amount)?;
        bank.change_asset_shares(-asset_shares_decrease)?;

        bank.check_utilization_ratio()?;

        let spl_withdraw_amount = current_asset_amount
            .checked_floor()
            .ok_or_else(math_error!())?;

        bank.collected_insurance_fees_outstanding = {
            current_asset_amount
                .checked_sub(spl_withdraw_amount)
                .ok_or_else(math_error!())?
                .checked_add(bank.collected_insurance_fees_outstanding.into())
                .ok_or_else(math_error!())?
                .into()
        };

        Ok(spl_withdraw_amount.to_num())
    }

    /// Repay existing liability in full - will error if there is no liability.
    pub fn repay_all(&mut self) -> MarginfiResult<u64> {
        let balance = &mut self.balance;
        let bank = &mut self.bank;

        let current_liability_amount =
            bank.get_liability_amount(balance.liability_shares.into())?;

        msg!(
            "Repaying all: {} of {} in {}",
            current_liability_amount,
            bank.mint,
            balance.bank_pk,
        );

        check!(
            current_liability_amount.is_positive(),
            MarginfiError::NoLiabilityFound
        );

        balance.close();
        let liability_shares_decrease = bank.get_liability_shares(current_liability_amount)?;
        bank.change_liability_shares(-liability_shares_decrease)?;

        let spl_deposit_amount = current_liability_amount
            .checked_ceil()
            .ok_or_else(math_error!())?;

        bank.collected_insurance_fees_outstanding = {
            spl_deposit_amount
                .checked_sub(current_liability_amount)
                .ok_or_else(math_error!())?
                .checked_add(bank.collected_insurance_fees_outstanding.into())
                .ok_or_else(math_error!())?
                .into()
        };

        Ok(spl_deposit_amount.to_num())
    }

    // ------------ Internal accounting logic

    fn increase_balance_internal(
        &mut self,
        balance_delta: I80F48,
        operation_type: BalanceIncreaseType,
    ) -> MarginfiResult {
        msg!(
            "Balance increase: {} of {} in {} (type: {:?})",
            balance_delta,
            self.bank.mint,
            self.balance.bank_pk,
            operation_type
        );

        let balance = &mut self.balance;
        let bank = &mut self.bank;

        let current_liability_shares: I80F48 = balance.liability_shares.into();
        let current_liability_amount = bank.get_liability_amount(current_liability_shares)?;

        let (liability_amount_decrease, asset_amount_increase) = (
            min(current_liability_amount, balance_delta),
            max(
                balance_delta
                    .checked_sub(current_liability_amount)
                    .ok_or_else(math_error!())?,
                I80F48::ZERO,
            ),
        );

        match operation_type {
            BalanceIncreaseType::RepayOnly => {
                check!(
                    asset_amount_increase.is_zero(),
                    MarginfiError::OperationRepayOnly
                );
            }
            BalanceIncreaseType::DepositOnly => {
                check!(
                    liability_amount_decrease.is_zero(),
                    MarginfiError::OperationDepositOnly
                );
            }
            BalanceIncreaseType::Any => {}
        }

        {
            let is_asset_amount_increasing = asset_amount_increase.is_positive();
            bank.assert_operational_mode(Some(is_asset_amount_increasing))?;
        }

        let asset_shares_increase = bank.get_asset_shares(asset_amount_increase)?;

        balance.change_asset_shares(asset_shares_increase)?;
        bank.change_asset_shares(asset_shares_increase)?;

        let liability_shares_decrease = bank.get_liability_shares(liability_amount_decrease)?;

        balance.change_liability_shares(-liability_shares_decrease)?;
        bank.change_liability_shares(-liability_shares_decrease)?;

        Ok(())
    }

    fn decrease_balance_internal(
        &mut self,
        balance_delta: I80F48,
        operation_type: BalanceDecreaseType,
    ) -> MarginfiResult {
        msg!(
            "Balance decrease: {} of {} in {} (type: {:?})",
            balance_delta,
            self.bank.mint,
            self.balance.bank_pk,
            operation_type
        );

        let balance = &mut self.balance;
        let bank = &mut self.bank;

        let current_asset_shares: I80F48 = balance.asset_shares.into();
        let current_asset_amount = bank.get_asset_amount(current_asset_shares)?;

        let (asset_amount_decrease, liability_amount_increase) = (
            min(current_asset_amount, balance_delta),
            max(
                balance_delta
                    .checked_sub(current_asset_amount)
                    .ok_or_else(math_error!())?,
                I80F48::ZERO,
            ),
        );

        match operation_type {
            BalanceDecreaseType::WithdrawOnly => {
                check!(
                    liability_amount_increase.is_zero(),
                    MarginfiError::OperationWithdrawOnly
                );
            }
            BalanceDecreaseType::BorrowOnly => {
                check!(
                    asset_amount_decrease.is_zero(),
                    MarginfiError::OperationBorrowOnly
                );
            }
            BalanceDecreaseType::Any => {}
        }

        {
            let is_liability_amount_increasing = liability_amount_increase.is_positive();
            bank.assert_operational_mode(Some(is_liability_amount_increasing))?;
        }

        let asset_shares_decrease = bank.get_asset_shares(asset_amount_decrease)?;

        balance.change_asset_shares(-asset_shares_decrease)?;
        bank.change_asset_shares(-asset_shares_decrease)?;

        let liability_shares_increase = bank.get_liability_shares(liability_amount_increase)?;

        balance.change_liability_shares(liability_shares_increase)?;
        bank.change_liability_shares(liability_shares_increase)?;

        bank.check_utilization_ratio()?;

        Ok(())
    }

    // ------------ SPL helpers

    pub fn deposit_spl_transfer<'b: 'c, 'c: 'b>(
        &self,
        amount: u64,
        accounts: Transfer<'b>,
        program: AccountInfo<'c>,
    ) -> MarginfiResult {
        self.bank.deposit_spl_transfer(amount, accounts, program)
    }

    pub fn withdraw_spl_transfer<'b: 'c, 'c: 'b>(
        &self,
        amount: u64,
        accounts: Transfer<'b>,
        program: AccountInfo<'c>,
        signer_seeds: &[&[&[u8]]],
    ) -> MarginfiResult {
        self.bank
            .withdraw_spl_transfer(amount, accounts, program, signer_seeds)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use fixed_macro::types::I80F48;

    #[test]
    fn test_calc_asset_value() {
        assert_eq!(
            calc_asset_value(I80F48!(10_000_000), I80F48!(1_000_000), 6, None).unwrap(),
            I80F48!(10_000_000)
        );

        assert_eq!(
            calc_asset_value(I80F48!(1_000_000_000), I80F48!(10_000_000), 9, None).unwrap(),
            I80F48!(10_000_000)
        );

        assert_eq!(
            calc_asset_value(I80F48!(1_000_000_000), I80F48!(10_000_000), 9, None).unwrap(),
            I80F48!(10_000_000)
        );
    }
}
