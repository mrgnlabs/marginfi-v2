use super::{
    marginfi_group::{Bank, RiskTier, WrappedI80F48},
    price::{OraclePriceFeedAdapter, OraclePriceType, PriceAdapter, PriceBias},
};
use crate::{
    assert_struct_size, check,
    constants::{
        BANKRUPT_THRESHOLD, EMISSIONS_FLAG_BORROW_ACTIVE, EMISSIONS_FLAG_LENDING_ACTIVE,
        EMPTY_BALANCE_THRESHOLD, EXP_10_I80F48, MAX_PRICE_AGE_SEC, MIN_EMISSIONS_START_TIME,
        SECONDS_PER_YEAR, ZERO_AMOUNT_THRESHOLD,
    },
    debug, math_error,
    prelude::{MarginfiError, MarginfiResult},
    utils::NumTraitsWithTolerance,
};
use anchor_lang::prelude::*;
use anchor_spl::token::Transfer;
use fixed::types::I80F48;
use std::{
    cmp::{max, min},
    ops::Not,
};
#[cfg(any(feature = "test", feature = "client"))]
use type_layout::TypeLayout;

assert_struct_size!(MarginfiAccount, 2304);
#[account(zero_copy(unsafe))]
#[repr(C)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
pub struct MarginfiAccount {
    pub group: Pubkey,                   // 32
    pub authority: Pubkey,               // 32
    pub lending_account: LendingAccount, // 1728
    /// The flas that indicates the state of the account.
    /// This is u64 bitfield, where each bit represents a flag.
    ///
    /// Flags:
    /// - DISABLED_FLAG = 1 << 0 = 1 - This flag indicates that the account is disabled,
    /// and no further actions can be taken on it.
    pub account_flags: u64, // 8
    pub _padding: [u64; 63],             // 8 * 63 = 512
}

pub const DISABLED_FLAG: u64 = 1 << 0;
pub const IN_FLASHLOAN_FLAG: u64 = 1 << 1;
pub const FLASHLOAN_ENABLED_FLAG: u64 = 1 << 2;

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

    pub fn set_flag(&mut self, flag: u64) {
        msg!("Setting account flag {:b}", flag);
        self.account_flags |= flag;
    }

    pub fn unset_flag(&mut self, flag: u64) {
        msg!("Unsetting account flag {:b}", flag);
        self.account_flags &= !flag;
    }

    pub fn get_flag(&self, flag: u64) -> bool {
        self.account_flags & flag != 0
    }
}

#[derive(Debug)]
pub enum BalanceIncreaseType {
    Any,
    RepayOnly,
    DepositOnly,
    BypassDepositLimit,
}

#[derive(Debug)]
pub enum BalanceDecreaseType {
    Any,
    WithdrawOnly,
    BorrowOnly,
    BypassBorrowLimit,
}

#[derive(Copy, Clone)]
pub enum RequirementType {
    Initial,
    Maintenance,
    Equity,
}

impl RequirementType {
    /// Get oracle price type for the requirement type.
    ///
    /// Initial and equity requirements use the time weighted price feed.
    /// Maintenance requirement uses the real time price feed, as its more accurate for triggering liquidations.
    pub fn get_oracle_price_type(&self) -> OraclePriceType {
        match self {
            RequirementType::Initial | RequirementType::Equity => OraclePriceType::TimeWeighted,
            RequirementType::Maintenance => OraclePriceType::RealTime,
        }
    }
}

pub struct BankAccountWithPriceFeed<'a, 'b> {
    bank: AccountInfo<'b>,
    price_feed: Box<MarginfiResult<OraclePriceFeedAdapter>>,
    balance: &'a Balance,
}

pub enum BalanceSide {
    Assets,
    Liabilities,
}

impl<'a, 'b> BankAccountWithPriceFeed<'a, 'b> {
    pub fn load(
        lending_account: &'a LendingAccount,
        remaining_ais: &[AccountInfo<'b>],
    ) -> MarginfiResult<Vec<BankAccountWithPriceFeed<'a, 'b>>> {
        let active_balances = lending_account
            .balances
            .iter()
            .filter(|balance| balance.active)
            .collect::<Vec<_>>();

        msg!("Expecting {} remaining accounts", active_balances.len() * 2);
        msg!("Got {} remaining accounts", remaining_ais.len());

        check!(
            active_balances.len() * 2 == remaining_ais.len(),
            MarginfiError::MissingPythOrBankAccount
        );

        let current_timestamp = Clock::get()?.unix_timestamp;

        active_balances
            .iter()
            .enumerate()
            .map(|(i, balance)| {
                let bank_index = i * 2;
                let oracle_ai_idx = bank_index + 1;

                let bank_ai = remaining_ais.get(bank_index).unwrap();

                check!(
                    balance.bank_pk.eq(bank_ai.key),
                    MarginfiError::InvalidBankAccount
                );

                let price_adapter = {
                    let oracle_ais = &remaining_ais[oracle_ai_idx..oracle_ai_idx + 1];
                    let bank_al = AccountLoader::<Bank>::try_from(bank_ai)?;
                    let bank = bank_al.load()?;

                    Box::new(OraclePriceFeedAdapter::try_from_bank_config(
                        &bank.config,
                        oracle_ais,
                        current_timestamp,
                        MAX_PRICE_AGE_SEC,
                    ))
                };

                Ok(BankAccountWithPriceFeed {
                    bank: bank_ai.clone(),
                    price_feed: price_adapter,
                    balance,
                })
            })
            .collect::<Result<Vec<_>>>()
    }

    #[inline(always)]
    /// Calculate the value of the assets and liabilities of the account in the form of (assets, liabilities)
    ///
    /// Nuances:
    /// 1. Maintenance requirement is calculated using the real time price feed.
    /// 2. Initial requirement is calculated using the time weighted price feed, if available.
    /// 3. Initial requirement is discounted by the initial discount, if enabled and the usd limit is exceeded.
    /// 4. Assets are only calculated for collateral risk tier.
    /// 5. Oracle errors are ignored for deposits in isolated risk tier.
    pub fn calc_weighted_assets_and_liabilities_values(
        &self,
        requirement_type: RequirementType,
    ) -> MarginfiResult<(I80F48, I80F48)> {
        match self.balance.get_side() {
            Some(side) => {
                let bank_al = AccountLoader::<Bank>::try_from(&self.bank)?;
                let bank = bank_al.load()?;
                match side {
                    BalanceSide::Assets => Ok((
                        self.calc_weighted_assets(requirement_type, &bank)?,
                        I80F48::ZERO,
                    )),
                    BalanceSide::Liabilities => Ok((
                        I80F48::ZERO,
                        self.calc_weighted_liabs(requirement_type, &bank)?,
                    )),
                }
            }
            None => Ok((I80F48::ZERO, I80F48::ZERO)),
        }
    }

    #[inline(always)]
    fn calc_weighted_assets(
        &self,
        requirement_type: RequirementType,
        bank: &Bank,
    ) -> MarginfiResult<I80F48> {
        match bank.config.risk_tier {
            RiskTier::Collateral => {
                let price_feed = self.try_get_price_feed()?;
                let mut asset_weight = bank
                    .config
                    .get_weight(requirement_type, BalanceSide::Assets);

                let lower_price = price_feed.get_price_of_type(
                    requirement_type.get_oracle_price_type(),
                    Some(PriceBias::Low),
                )?;

                if matches!(requirement_type, RequirementType::Initial) {
                    if let Some(discount) =
                        bank.maybe_get_asset_weight_init_discount(lower_price)?
                    {
                        asset_weight = asset_weight
                            .checked_mul(discount)
                            .ok_or_else(math_error!())?;
                    }
                }

                calc_value(
                    bank.get_asset_amount(self.balance.asset_shares.into())?,
                    lower_price,
                    bank.mint_decimals,
                    Some(asset_weight),
                )
            }
            RiskTier::Isolated => Ok(I80F48::ZERO),
        }
    }

    #[inline(always)]
    fn calc_weighted_liabs(
        &self,
        requirement_type: RequirementType,
        bank: &Bank,
    ) -> MarginfiResult<I80F48> {
        let price_feed = self.try_get_price_feed()?;
        let liability_weight = bank
            .config
            .get_weight(requirement_type, BalanceSide::Liabilities);

        let higher_price = price_feed.get_price_of_type(
            requirement_type.get_oracle_price_type(),
            Some(PriceBias::High),
        )?;

        calc_value(
            bank.get_liability_amount(self.balance.liability_shares.into())?,
            higher_price,
            bank.mint_decimals,
            Some(liability_weight),
        )
    }

    fn try_get_price_feed(&self) -> MarginfiResult<&OraclePriceFeedAdapter> {
        match self.price_feed.as_ref() {
            Ok(a) => Ok(a),
            Err(_) => Err(MarginfiError::StaleOracle)?,
        }
    }

    #[inline]
    pub fn is_empty(&self, side: BalanceSide) -> bool {
        self.balance.is_empty(side)
    }
}

/// Calculate the value of an asset, given its quantity with a decimal exponent, and a price with a decimal exponent, and an optional weight.
#[inline]
pub fn calc_value(
    amount: I80F48,
    price: I80F48,
    mint_decimals: u8,
    weight: Option<I80F48>,
) -> MarginfiResult<I80F48> {
    if amount == I80F48::ZERO {
        return Ok(I80F48::ZERO);
    }

    let scaling_factor = EXP_10_I80F48[mint_decimals as usize];

    let weighted_asset_amount = if let Some(weight) = weight {
        amount.checked_mul(weight).unwrap()
    } else {
        amount
    };

    #[cfg(target_os = "solana")]
    msg!(
        "weighted_asset_qt: {}, price: {}, expo: {}",
        weighted_asset_amount,
        price,
        mint_decimals
    );

    let value = weighted_asset_amount
        .checked_mul(price)
        .ok_or_else(math_error!())?
        .checked_div(scaling_factor)
        .ok_or_else(math_error!())?;

    Ok(value)
}

#[inline]
pub fn calc_amount(value: I80F48, price: I80F48, mint_decimals: u8) -> MarginfiResult<I80F48> {
    let scaling_factor = EXP_10_I80F48[mint_decimals as usize];

    let qt = value
        .checked_mul(scaling_factor)
        .ok_or_else(math_error!())?
        .checked_div(price)
        .ok_or_else(math_error!())?;

    Ok(qt)
}

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

pub struct RiskEngine<'a, 'b> {
    marginfi_account: &'a MarginfiAccount,
    bank_accounts_with_price: Vec<BankAccountWithPriceFeed<'a, 'b>>,
}

impl<'a, 'b> RiskEngine<'a, 'b> {
    pub fn new(
        marginfi_account: &'a MarginfiAccount,
        remaining_ais: &[AccountInfo<'b>],
    ) -> MarginfiResult<Self> {
        check!(
            !marginfi_account.get_flag(IN_FLASHLOAN_FLAG),
            MarginfiError::AccountInFlashloan
        );

        Self::new_no_flashloan_check(marginfi_account, remaining_ais)
    }

    /// Internal constructor used either after manually checking account is not in a flashloan,
    /// or explicity checking health for flashloan enabled actions.
    fn new_no_flashloan_check(
        marginfi_account: &'a MarginfiAccount,
        remaining_ais: &[AccountInfo<'b>],
    ) -> MarginfiResult<Self> {
        let bank_accounts_with_price =
            BankAccountWithPriceFeed::load(&marginfi_account.lending_account, remaining_ais)?;

        Ok(Self {
            marginfi_account,
            bank_accounts_with_price,
        })
    }

    /// Checks account is healty after performing actions that increase risk (removing liquidity).
    ///
    /// `IN_FLASHLOAN_FLAG` behaviour.
    /// - Health check is skipped.
    /// - `remaining_ais` can be an empty vec.
    pub fn check_account_init_health(
        marginfi_account: &'a MarginfiAccount,
        remaining_ais: &[AccountInfo<'b>],
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

            total_assets = total_assets.checked_add(assets).ok_or_else(math_error!())?;
            total_liabilities = total_liabilities
                .checked_add(liabilities)
                .ok_or_else(math_error!())?;
        }

        Ok((total_assets, total_liabilities))
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

    fn check_account_health(&self, requirement_type: RiskRequirementType) -> MarginfiResult {
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

        msg!(
            "pre_liquidation_health: {} ({} - {})",
            account_health,
            assets,
            liabs
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

        msg!(
            "account_health: {} ({} - {}), pre_liquidation_health: {}",
            account_health,
            assets,
            liabs,
            pre_liquidation_health,
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

    fn check_account_risk_tiers(&self) -> MarginfiResult {
        let balances_with_liablities = self
            .bank_accounts_with_price
            .iter()
            .filter(|a| a.balance.is_empty(BalanceSide::Liabilities).not());

        let n_balances_with_liablities = balances_with_liablities.clone().count();

        let is_in_isolated_risk_tier = balances_with_liablities.clone().any(|a| {
            AccountLoader::<Bank>::try_from(&a.bank)
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

const MAX_LENDING_ACCOUNT_BALANCES: usize = 16;

assert_struct_size!(LendingAccount, 1728);
#[zero_copy(unsafe)]
#[repr(C)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
pub struct LendingAccount {
    pub balances: [Balance; MAX_LENDING_ACCOUNT_BALANCES], // 104 * 16 = 1664
    pub _padding: [u64; 8],                                // 8 * 8 = 64
}

impl LendingAccount {
    pub fn get_first_empty_balance(&self) -> Option<usize> {
        self.balances.iter().position(|b| !b.active)
    }
}

#[cfg(any(feature = "test", feature = "client"))]
impl LendingAccount {
    pub fn get_balance(&self, bank_pk: &Pubkey) -> Option<&Balance> {
        self.balances
            .iter()
            .find(|balance| balance.active && balance.bank_pk.eq(bank_pk))
    }

    pub fn get_active_balances_iter(&self) -> impl Iterator<Item = &Balance> {
        self.balances.iter().filter(|b| b.active)
    }
}

assert_struct_size!(Balance, 104);
#[zero_copy(unsafe)]
#[repr(C)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
pub struct Balance {
    pub active: bool,
    pub bank_pk: Pubkey,
    pub asset_shares: WrappedI80F48,
    pub liability_shares: WrappedI80F48,
    pub emissions_outstanding: WrappedI80F48,
    pub last_update: u64,
    pub _padding: [u64; 1],
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

    pub fn close(&mut self) -> MarginfiResult {
        check!(
            I80F48::from(self.emissions_outstanding) < I80F48::ONE,
            MarginfiError::CannotCloseOutstandingEmissions
        );

        *self = Self::empty_deactivated();

        Ok(())
    }

    pub fn get_side(&self) -> Option<BalanceSide> {
        let asset_shares = I80F48::from(self.asset_shares);
        let liability_shares = I80F48::from(self.liability_shares);

        assert!(
            asset_shares < EMPTY_BALANCE_THRESHOLD || liability_shares < EMPTY_BALANCE_THRESHOLD
        );

        if I80F48::from(self.asset_shares) >= EMPTY_BALANCE_THRESHOLD {
            Some(BalanceSide::Assets)
        } else if I80F48::from(self.liability_shares) >= EMPTY_BALANCE_THRESHOLD {
            Some(BalanceSide::Liabilities)
        } else {
            None
        }
    }

    pub fn empty_deactivated() -> Self {
        Balance {
            active: false,
            bank_pk: Pubkey::default(),
            asset_shares: WrappedI80F48::from(I80F48::ZERO),
            liability_shares: WrappedI80F48::from(I80F48::ZERO),
            emissions_outstanding: WrappedI80F48::from(I80F48::ZERO),
            last_update: 0,
            _padding: [0; 1],
        }
    }
}

pub struct BankAccountWrapper<'a> {
    pub balance: &'a mut Balance,
    pub bank: &'a mut Bank,
}

impl<'a> BankAccountWrapper<'a> {
    // Find existing user lending account balance by bank address.
    pub fn find(
        bank_pk: &Pubkey,
        bank: &'a mut Bank,
        lending_account: &'a mut LendingAccount,
    ) -> MarginfiResult<BankAccountWrapper<'a>> {
        let balance = lending_account
            .balances
            .iter_mut()
            .find(|balance| balance.active && balance.bank_pk.eq(bank_pk))
            .ok_or_else(|| error!(MarginfiError::BankAccoutNotFound))?;

        Ok(Self { balance, bank })
    }

    // Find existing user lending account balance by bank address.
    // Create it if not found.
    pub fn find_or_create(
        bank_pk: &Pubkey,
        bank: &'a mut Bank,
        lending_account: &'a mut LendingAccount,
    ) -> MarginfiResult<BankAccountWrapper<'a>> {
        let balance_index = lending_account
            .balances
            .iter()
            .position(|balance| balance.active && balance.bank_pk.eq(bank_pk));

        match balance_index {
            Some(balance_index) => {
                let balance = lending_account
                    .balances
                    .get_mut(balance_index)
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
                    emissions_outstanding: I80F48::ZERO.into(),
                    last_update: Clock::get()?.unix_timestamp as u64,
                    _padding: [0; 1],
                };

                Ok(Self {
                    balance: lending_account.balances.get_mut(empty_index).unwrap(),
                    bank,
                })
            }
        }
    }

    // ------------ Borrow / Lend primitives

    /// Deposit an asset, will repay any outstanding liabilities.
    pub fn deposit(&mut self, amount: I80F48) -> MarginfiResult {
        self.increase_balance_internal(amount, BalanceIncreaseType::Any)
    }

    /// Repay a liability, will error if there is not enough liability - depositing is not allowed.
    pub fn repay(&mut self, amount: I80F48) -> MarginfiResult {
        self.increase_balance_internal(amount, BalanceIncreaseType::RepayOnly)
    }

    /// Withdraw an asset, will error if there is not enough asset - borrowing is not allowed.
    pub fn withdraw(&mut self, amount: I80F48) -> MarginfiResult {
        self.decrease_balance_internal(amount, BalanceDecreaseType::WithdrawOnly)
    }

    /// Incur a borrow, will withdraw any existing assets.
    pub fn borrow(&mut self, amount: I80F48) -> MarginfiResult {
        self.decrease_balance_internal(amount, BalanceDecreaseType::Any)
    }

    // ------------ Hybrid operations for seamless repay + deposit / withdraw + borrow

    /// Repay liability and deposit/increase asset depending on
    /// the specified deposit amount and the existing balance.
    pub fn increase_balance(&mut self, amount: I80F48) -> MarginfiResult {
        self.increase_balance_internal(amount, BalanceIncreaseType::Any)
    }

    pub fn increase_balance_in_liquidation(&mut self, amount: I80F48) -> MarginfiResult {
        self.increase_balance_internal(amount, BalanceIncreaseType::BypassDepositLimit)
    }

    /// Withdraw asset and create/increase liability depending on
    /// the specified deposit amount and the existing balance.
    pub fn decrease_balance(&mut self, amount: I80F48) -> MarginfiResult {
        self.decrease_balance_internal(amount, BalanceDecreaseType::Any)
    }

    /// Withdraw asset and create/increase liability depending on
    /// the specified deposit amount and the existing balance.
    ///
    /// This function will also bypass borrow limits
    /// so liquidations can happen in banks with maxed out borrows.
    pub fn decrease_balance_in_liquidation(&mut self, amount: I80F48) -> MarginfiResult {
        self.decrease_balance_internal(amount, BalanceDecreaseType::BypassBorrowLimit)
    }

    /// Withdraw existing asset in full - will error if there is no asset.
    pub fn withdraw_all(&mut self) -> MarginfiResult<u64> {
        self.claim_emissions(Clock::get()?.unix_timestamp as u64)?;

        let balance = &mut self.balance;
        let bank = &mut self.bank;

        bank.assert_operational_mode(None)?;

        let total_asset_shares: I80F48 = balance.asset_shares.into();
        let current_asset_amount = bank.get_asset_amount(total_asset_shares)?;
        let current_liability_amount =
            bank.get_liability_amount(balance.liability_shares.into())?;

        debug!("Withdrawing all: {}", current_asset_amount);

        check!(
            current_asset_amount.is_positive_with_tolerance(ZERO_AMOUNT_THRESHOLD),
            MarginfiError::NoAssetFound
        );

        check!(
            current_liability_amount.is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
            MarginfiError::NoAssetFound
        );

        balance.close()?;
        bank.change_asset_shares(-total_asset_shares, false)?;

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

        Ok(spl_withdraw_amount
            .checked_to_num()
            .ok_or_else(math_error!())?)
    }

    /// Repay existing liability in full - will error if there is no liability.
    pub fn repay_all(&mut self) -> MarginfiResult<u64> {
        self.claim_emissions(Clock::get()?.unix_timestamp as u64)?;

        let balance = &mut self.balance;
        let bank = &mut self.bank;

        bank.assert_operational_mode(None)?;

        let total_liability_shares: I80F48 = balance.liability_shares.into();
        let current_liability_amount = bank.get_liability_amount(total_liability_shares)?;
        let current_asset_amount = bank.get_asset_amount(balance.asset_shares.into())?;

        debug!("Repaying all: {}", current_liability_amount,);

        check!(
            current_liability_amount.is_positive_with_tolerance(ZERO_AMOUNT_THRESHOLD),
            MarginfiError::NoLiabilityFound
        );

        check!(
            current_asset_amount.is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
            MarginfiError::NoLiabilityFound
        );

        balance.close()?;
        bank.change_liability_shares(-total_liability_shares, false)?;

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

        Ok(spl_deposit_amount
            .checked_to_num()
            .ok_or_else(math_error!())?)
    }

    pub fn close_balance(&mut self) -> MarginfiResult<()> {
        self.claim_emissions(Clock::get()?.unix_timestamp as u64)?;

        let balance = &mut self.balance;
        let bank = &mut self.bank;

        let current_liability_amount =
            bank.get_liability_amount(balance.liability_shares.into())?;
        let current_asset_amount = bank.get_asset_amount(balance.asset_shares.into())?;

        check!(
            current_liability_amount.is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
            MarginfiError::IllegalBalanceState,
            "Balance has existing debt"
        );

        check!(
            current_asset_amount.is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
            MarginfiError::IllegalBalanceState,
            "Balance has existing assets"
        );

        balance.close()?;

        Ok(())
    }

    // ------------ Internal accounting logic

    fn increase_balance_internal(
        &mut self,
        balance_delta: I80F48,
        operation_type: BalanceIncreaseType,
    ) -> MarginfiResult {
        msg!(
            "Balance increase: {} (type: {:?})",
            balance_delta,
            operation_type
        );

        self.claim_emissions(Clock::get()?.unix_timestamp as u64)?;

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
                    asset_amount_increase.is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
                    MarginfiError::OperationRepayOnly
                );
            }
            BalanceIncreaseType::DepositOnly => {
                check!(
                    liability_amount_decrease.is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
                    MarginfiError::OperationDepositOnly
                );
            }
            BalanceIncreaseType::Any | BalanceIncreaseType::BypassDepositLimit => {}
        }

        {
            let is_asset_amount_increasing =
                asset_amount_increase.is_positive_with_tolerance(ZERO_AMOUNT_THRESHOLD);
            bank.assert_operational_mode(Some(is_asset_amount_increasing))?;
        }

        let asset_shares_increase = bank.get_asset_shares(asset_amount_increase)?;
        balance.change_asset_shares(asset_shares_increase)?;
        bank.change_asset_shares(
            asset_shares_increase,
            matches!(operation_type, BalanceIncreaseType::BypassDepositLimit),
        )?;

        let liability_shares_decrease = bank.get_liability_shares(liability_amount_decrease)?;
        // TODO: Use `IncreaseType` to skip certain balance updates, and save on compute.
        balance.change_liability_shares(-liability_shares_decrease)?;
        bank.change_liability_shares(-liability_shares_decrease, true)?;

        Ok(())
    }

    fn decrease_balance_internal(
        &mut self,
        balance_delta: I80F48,
        operation_type: BalanceDecreaseType,
    ) -> MarginfiResult {
        msg!(
            "Balance decrease: {} of (type: {:?})",
            balance_delta,
            operation_type
        );

        self.claim_emissions(Clock::get()?.unix_timestamp as u64)?;

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
                    liability_amount_increase.is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
                    MarginfiError::OperationWithdrawOnly
                );
            }
            BalanceDecreaseType::BorrowOnly => {
                check!(
                    asset_amount_decrease.is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
                    MarginfiError::OperationBorrowOnly
                );
            }
            _ => {}
        }

        {
            let is_liability_amount_increasing =
                liability_amount_increase.is_positive_with_tolerance(ZERO_AMOUNT_THRESHOLD);
            bank.assert_operational_mode(Some(is_liability_amount_increasing))?;
        }

        let asset_shares_decrease = bank.get_asset_shares(asset_amount_decrease)?;
        balance.change_asset_shares(-asset_shares_decrease)?;
        bank.change_asset_shares(-asset_shares_decrease, false)?;

        let liability_shares_increase = bank.get_liability_shares(liability_amount_increase)?;
        balance.change_liability_shares(liability_shares_increase)?;
        bank.change_liability_shares(
            liability_shares_increase,
            matches!(operation_type, BalanceDecreaseType::BypassBorrowLimit),
        )?;

        bank.check_utilization_ratio()?;

        Ok(())
    }

    /// Claim any unclaimed emissions and add them to the outstanding emissions amount.
    pub fn claim_emissions(&mut self, current_timestamp: u64) -> MarginfiResult {
        if let Some(balance_amount) = match (
            self.balance.get_side(),
            self.bank.get_emissions_flag(EMISSIONS_FLAG_LENDING_ACTIVE),
            self.bank.get_emissions_flag(EMISSIONS_FLAG_BORROW_ACTIVE),
        ) {
            (Some(BalanceSide::Assets), true, _) => Some(
                self.bank
                    .get_asset_amount(self.balance.asset_shares.into())?,
            ),
            (Some(BalanceSide::Liabilities), _, true) => Some(
                self.bank
                    .get_liability_amount(self.balance.liability_shares.into())?,
            ),
            _ => None,
        } {
            let last_update = if self.balance.last_update < MIN_EMISSIONS_START_TIME {
                current_timestamp
            } else {
                self.balance.last_update
            };
            let period = I80F48::from_num(
                current_timestamp
                    .checked_sub(last_update)
                    .ok_or_else(math_error!())?,
            );
            let emissions_rate = I80F48::from_num(self.bank.emissions_rate);
            let emissions = period
                .checked_mul(balance_amount)
                .ok_or_else(math_error!())?
                .checked_div(EXP_10_I80F48[self.bank.mint_decimals as usize])
                .ok_or_else(math_error!())?
                .checked_mul(emissions_rate)
                .ok_or_else(math_error!())?
                .checked_div(SECONDS_PER_YEAR)
                .ok_or_else(math_error!())?;

            let emissions_real = min(emissions, I80F48::from(self.bank.emissions_remaining));

            msg!(
                "Emitting {} ({} calculated) for period {}s",
                emissions_real,
                emissions,
                period
            );

            msg!(
                "Outstanding emissions: {}",
                I80F48::from(self.balance.emissions_outstanding)
            );

            self.balance.emissions_outstanding = {
                I80F48::from(self.balance.emissions_outstanding)
                    .checked_add(emissions_real)
                    .ok_or_else(math_error!())?
            }
            .into();
            self.bank.emissions_remaining = {
                I80F48::from(self.bank.emissions_remaining)
                    .checked_sub(emissions_real)
                    .ok_or_else(math_error!())?
            }
            .into();
        }

        self.balance.last_update = current_timestamp;

        Ok(())
    }

    /// Claim any outstanding emissions, and return the max amount that can be withdrawn.
    pub fn settle_emissions_and_get_transfer_amount(&mut self) -> MarginfiResult<u64> {
        self.claim_emissions(Clock::get()?.unix_timestamp as u64)?;

        let outstanding_emissions_floored = I80F48::from(self.balance.emissions_outstanding)
            .checked_floor()
            .ok_or_else(math_error!())?;
        let new_outstanding_amount = I80F48::from(self.balance.emissions_outstanding)
            .checked_sub(outstanding_emissions_floored)
            .ok_or_else(math_error!())?;

        self.balance.emissions_outstanding = new_outstanding_amount.into();

        Ok(outstanding_emissions_floored
            .checked_to_num::<u64>()
            .ok_or_else(math_error!())?)
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
            calc_value(I80F48!(10_000_000), I80F48!(1_000_000), 6, None).unwrap(),
            I80F48!(10_000_000)
        );

        assert_eq!(
            calc_value(I80F48!(1_000_000_000), I80F48!(10_000_000), 9, None).unwrap(),
            I80F48!(10_000_000)
        );

        assert_eq!(
            calc_value(I80F48!(1_000_000_000), I80F48!(10_000_000), 9, None).unwrap(),
            I80F48!(10_000_000)
        );
    }
}
