use super::marginfi_group::{Bank, WrappedI80F48};
use crate::{
    check,
    constants::{CONF_INTERVAL_MULTIPLE, MAX_PRICE_AGE_SEC},
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
    pub in_flashloan: bool,
}

impl MarginfiAccount {
    /// Set the initial data for the marginfi account.
    pub fn initialize(&mut self, group: Pubkey, authority: Pubkey) {
        assert!(self.in_flashloan.not());

        self.authority = authority;
        self.group = group;
    }

    pub fn get_remaining_accounts_len(&self) -> usize {
        self.lending_account
            .balances
            .iter()
            .filter(|b| b.active)
            .count()
            * 2
    }

    pub fn start_flashloan(&mut self) {
        assert!(self.in_flashloan.not());
        self.in_flashloan = true;
    }

    pub fn end_flashloan(&mut self) {
        assert!(self.in_flashloan);
        self.in_flashloan = false;
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

pub enum WeightType {
    Initial,
    Maintenance,
}

pub struct BankAccountWithPriceFeed<'a> {
    bank: Bank,
    price_feed: PriceFeed,
    balance: &'a Balance,
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

        let deposits_qt = self
            .bank
            .get_deposit_amount(self.balance.deposit_shares.into())?;
        let liabilities_qt = self
            .bank
            .get_deposit_amount(self.balance.liability_shares.into())?;
        let (deposit_weight, liability_weight) = self.bank.config.get_weights(weight_type); // TODO: asset-specific weights

        let mint_decimals = self.bank.mint_decimals;

        Ok((
            calc_asset_value(
                deposits_qt,
                worst_price,
                mint_decimals,
                Some(deposit_weight),
            )?,
            calc_asset_value(
                liabilities_qt,
                best_price,
                mint_decimals,
                Some(liability_weight),
            )?,
        ))
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

#[inline]
/// Calculate the value of an asset, given its quantity with a decimal exponent, and a price with a decimal exponent, and an optional weight.
pub fn calc_asset_value(
    asset_quantity: I80F48,
    price: I80F48,
    mint_decimals: u8,
    weight: Option<I80F48>,
) -> MarginfiResult<I80F48> {
    if asset_quantity == I80F48::ZERO {
        return Ok(I80F48::ZERO);
    }

    let scaling_factor = EXP_10_I80F48[mint_decimals as usize];

    let weighted_asset_qt = if let Some(weight) = weight {
        asset_quantity.checked_mul(weight).unwrap()
    } else {
        asset_quantity
    };

    msg!(
        "weighted_asset_qt: {}, price: {}, expo: {}",
        weighted_asset_qt,
        price,
        mint_decimals
    );

    let asset_value = weighted_asset_qt
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
    in_flashloan: bool,
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
            in_flashloan: marginfi_account.in_flashloan,
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

    pub fn check_account_health(&self, requirement_type: RiskRequirementType) -> MarginfiResult {
        if self.in_flashloan {
            msg!("check_health: in_flashloan, skipping check");
            return Ok(());
        }

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

    /// Check that the account is at most at the maintenance requirement level post liquidation.
    /// This check is used to ensure two things in the liquidation process:
    /// 1. Liquidatee account was below the maintenance requirement level before liquidation (as health can only increase, because liquidations always pay down liabilities)
    /// 2. Liquidator didn't liquidate too many assets that would result in unnecessary loss for the liquidatee.
    ///
    /// This check works on the assumption that the liquidation always results in a reduction of risk.
    ///
    /// 1. We check that the paid off liability is not zero. Assuming the liquidation always pays off some liability, this ensures that the liquidation was not too large.
    /// 2. We check that the account is still at most at the maintenance requirement level. This ensures that the liquidation was not too large overall.
    pub fn check_post_liquidation_account_health(&self, bank_pk: &Pubkey) -> MarginfiResult {
        let liability_bank_balance = self
            .bank_accounts_with_price
            .iter()
            .find(|a| a.balance.bank_pk == *bank_pk)
            .unwrap();

        check!(
            liability_bank_balance.balance.liability_shares.value != 0,
            MarginfiError::AccountIllegalPostLiquidationState
        );

        let (total_weighted_assets, total_weighted_liabilities) =
            self.get_account_health_components(RiskRequirementType::Maintenance)?;

        msg!(
            "check_post_liq: assets {} - liabs: {}",
            total_weighted_assets,
            total_weighted_liabilities
        );

        check!(
            total_weighted_assets <= total_weighted_liabilities,
            MarginfiError::AccountIllegalPostLiquidationState
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
    pub fn get_balance(&self, mint_pk: &Pubkey, bank: &Bank) -> Option<&Balance> {
        self.balances
            .iter()
            .find(|balance| balance.active && bank.mint.eq(mint_pk))
    }

    pub fn get_first_empty_balance(&self) -> Option<usize> {
        self.balances.iter().position(|b| !b.active)
    }

    pub fn get_active_balances_iter(&self) -> impl Iterator<Item = &Balance> {
        self.balances.iter().filter(|b| b.active)
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
    pub deposit_shares: WrappedI80F48,
    pub liability_shares: WrappedI80F48,
}

impl Balance {
    pub fn change_deposit_shares(&mut self, delta: I80F48) -> MarginfiResult {
        let deposit_shares: I80F48 = self.deposit_shares.into();
        self.deposit_shares = deposit_shares
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
}

pub struct BankAccountWrapper<'a> {
    balance: &'a mut Balance,
    bank: &'a mut Bank,
}

impl<'a> BankAccountWrapper<'a> {
    pub fn find_or_create<'b>(
        bank_pk: &Pubkey,
        bank: &'a mut Bank,
        lending_account: &'a mut LendingAccount,
    ) -> MarginfiResult<BankAccountWrapper<'a>> {
        // Find the user lending account balance by `bank_index`.
        // The balance account might not exist.
        let balance_index = lending_account
            .get_active_balances_iter()
            .position(|balance| balance.bank_pk.eq(bank_pk));

        let balance = if let Some(index) = balance_index {
            lending_account
                .balances // active_balances?
                .get_mut(index)
                .ok_or_else(|| error!(MarginfiError::LendingAccountBalanceNotFound))?
        } else {
            let empty_index = lending_account
                .get_first_empty_balance()
                .ok_or_else(|| error!(MarginfiError::LendingAccountBalanceSlotsFull))?;

            lending_account.balances[empty_index] = Balance {
                active: true,
                bank_pk: *bank_pk,
                deposit_shares: I80F48::ZERO.into(),
                liability_shares: I80F48::ZERO.into(),
            };

            lending_account.balances.get_mut(empty_index).unwrap()
        };

        Ok(Self { balance, bank })
    }

    pub fn account_deposit(&mut self, amount: I80F48) -> MarginfiResult {
        let balance = &mut self.balance;
        let bank = &mut self.bank;

        msg!("Account deposit: {} to {}", amount, balance.bank_pk);

        let liability_value = bank.get_liability_amount(balance.liability_shares.into())?;

        let (deposit_value_delta, liability_replay_value_delta) = (
            max(
                amount
                    .checked_sub(liability_value)
                    .ok_or_else(math_error!())?,
                I80F48::ZERO,
            ),
            min(liability_value, amount),
        );

        {
            let is_deposit_increasing = !deposit_value_delta.is_zero();
            bank.assert_operational_mode(Some(is_deposit_increasing))?;
        }

        let deposit_shares_delta = bank.get_deposit_shares(deposit_value_delta)?;

        balance.change_deposit_shares(deposit_shares_delta)?;
        bank.change_deposit_shares(deposit_shares_delta)?;

        let liability_shares_delta = bank.get_liability_shares(liability_replay_value_delta)?;

        balance.change_liability_shares(-liability_shares_delta)?;
        bank.change_liability_shares(-liability_shares_delta)?;

        Ok(())
    }

    /// Borrow an asset, will withdraw existing deposits if they exist.
    pub fn account_borrow(&mut self, amount: I80F48) -> MarginfiResult {
        self.account_credit_asset(amount, true)
    }

    /// Withdraw a deposit, will error if there is not enough deposit.
    /// Borrowing is not allowed.
    pub fn account_withdraw(&mut self, amount: I80F48) -> MarginfiResult {
        self.account_credit_asset(amount, false)
    }

    fn account_credit_asset(&mut self, amount: I80F48, allow_borrow: bool) -> MarginfiResult {
        msg!(
            "Account remove: {} of {} (borrow: {})",
            amount,
            self.bank.mint,
            allow_borrow
        );
        let balance = &mut self.balance;
        let bank = &mut self.bank;

        let deposit_shares: I80F48 = balance.deposit_shares.into();

        let deposit_value = bank.get_deposit_amount(deposit_shares)?;

        let (deposit_remove_value_delta, liability_value_delta) = (
            min(deposit_value, amount),
            max(
                amount
                    .checked_sub(deposit_value)
                    .ok_or_else(math_error!())?,
                I80F48::ZERO,
            ),
        );

        {
            let is_liability_increasing = liability_value_delta.is_zero().not();

            check!(
                allow_borrow || !is_liability_increasing,
                MarginfiError::BorrowingNotAllowed
            );

            bank.assert_operational_mode(Some(is_liability_increasing))?;
        }

        let deposit_shares_delta = bank.get_deposit_shares(deposit_remove_value_delta)?;
        balance.change_deposit_shares(-deposit_shares_delta)?;
        bank.change_deposit_shares(-deposit_shares_delta)?;

        let liability_shares_delta = bank.get_liability_shares(liability_value_delta)?;
        balance.change_liability_shares(liability_shares_delta)?;
        bank.change_liability_shares(liability_shares_delta)?;

        bank.check_utilization_ratio()?;

        Ok(())
    }

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
