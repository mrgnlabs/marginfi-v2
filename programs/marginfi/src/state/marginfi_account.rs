use super::marginfi_group::{Bank, LendingPool, MarginfiGroup, WrappedI80F48};
use crate::{
    check, math_error,
    prelude::{MarginfiError, MarginfiResult},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Transfer};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use pyth_sdk_solana::{state::PriceAccount, Price, PriceFeed};
use std::{
    cmp::{max, min},
    collections::{hash_map::RandomState, BTreeSet, HashMap},
    ops::DerefMut,
    ptr::read_unaligned,
};

#[account(zero_copy)]
pub struct MarginfiAccount {
    pub group: Pubkey,
    pub owner: Pubkey,
    pub lending_account: LendingAccount,
}

impl MarginfiAccount {
    /// Set the initial data for the marginfi account.
    pub fn initialize(&mut self, group: Pubkey, owner: Pubkey) {
        self.owner = owner;
        self.group = group;
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

const EXPONENT: i32 = 6;

/// Convert a price `price.price` with decimal exponent `price.expo` to an I80F48 representation with exponent 6.
pub fn pyth_price_to_i80f48(price: &Price) -> MarginfiResult<I80F48> {
    let pyth_price = price.price;
    let pyth_expo = price.expo;

    let expo_delta = EXPONENT - pyth_expo;
    let expo_scale = EXP_10_I80F48[expo_delta.abs() as usize];

    let mut price = I80F48::from_num(pyth_price);

    let price = if expo_delta < 0 {
        price.checked_div(expo_scale).ok_or_else(math_error!())?
    } else {
        price.checked_mul(expo_scale).ok_or_else(math_error!())?
    };

    Ok(price)
}

pub enum WeightType {
    Initial,
    Maintenance,
}

pub struct BankAccountWithPriceFeed<'a> {
    bank: &'a Bank,
    price_feed: PriceFeed,
    balance: &'a Balance,
}

impl<'a> BankAccountWithPriceFeed<'a> {
    pub fn load(
        lending_account: &'a LendingAccount,
        lending_pool: &'a LendingPool,
        pyth_accounts: &[AccountInfo],
    ) -> MarginfiResult<Vec<BankAccountWithPriceFeed<'a>>> {
        let pyth_accounts: HashMap<Pubkey, &AccountInfo, RandomState> =
            HashMap::from_iter(pyth_accounts.iter().map(|a| (a.key(), a)));

        lending_account
            .balances
            .iter()
            .filter_map(|b| b.as_ref())
            .map(|balance| {
                let bank = lending_pool
                    .banks
                    .get(balance.bank_index as usize)
                    .expect("Bank not found");
                let pyth_account = pyth_accounts
                    .get(&bank.config.pyth_oracle)
                    .expect("Pyth oracle not found");

                let pyth_data = pyth_account.try_borrow_data()?;
                let price_account = bytemuck::try_from_bytes::<PriceAccount>(&pyth_data.as_ref())
                    .expect("Invalid pyth data");
                let price_feed = price_account.to_price_feed(pyth_account.key);

                Ok(BankAccountWithPriceFeed {
                    bank,
                    price_feed,
                    balance,
                })
            })
            .collect::<Result<Vec<_>>>()
    }

    pub fn get_weighted_assets_and_liabilities(
        &self,
        weight_type: WeightType,
    ) -> MarginfiResult<(I80F48, I80F48)> {
        // TODO: Expire price, and check confidence interval
        let price = self.price_feed.get_price_unchecked();

        let deposits = self
            .bank
            .get_deposit_value(self.balance.deposit_shares.into())?;
        let liabilities = self
            .bank
            .get_deposit_value(self.balance.liability_shares.into())?;

        let (deposit_weight, liability_weight) = self.bank.config.get_weights(weight_type);

        Ok((
            {
                let weighted_deposits = deposits
                    .checked_mul(deposit_weight)
                    .ok_or_else(math_error!())?;

                weighted_deposits
                    .checked_mul(I80F48::from_num(price.price))
                    .ok_or_else(math_error!())?
                    .checked_div(EXP_10_I80F48[price.expo.abs() as usize])
                    .ok_or_else(math_error!())?
            },
            {
                let weighted_liabilities = liabilities
                    .checked_mul(liability_weight)
                    .ok_or_else(math_error!())?;

                weighted_liabilities
                    .checked_mul(I80F48::from_num(price.price))
                    .ok_or_else(math_error!())?
                    .checked_div(EXP_10_I80F48[price.expo.abs() as usize])
                    .ok_or_else(math_error!())?
            },
        ))
    }
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
    margin_group: &'a MarginfiGroup,
    marginfi_account: &'a MarginfiAccount,
    bank_accounts_with_price: Vec<BankAccountWithPriceFeed<'a>>,
}

impl<'a> RiskEngine<'a> {
    pub fn new(
        margin_group: &'a MarginfiGroup,
        marginfi_account: &'a MarginfiAccount,
        oracle_ais: &[AccountInfo],
    ) -> MarginfiResult<Self> {
        let lending_pool = unsafe {
            std::ptr::addr_of!(margin_group.lending_pool)
                .as_ref()
                .unwrap()
        };

        let bank_accounts_with_price = BankAccountWithPriceFeed::load(
            &marginfi_account.lending_account,
            &lending_pool,
            oracle_ais,
        )?;

        Ok(Self {
            margin_group,
            marginfi_account,
            bank_accounts_with_price: bank_accounts_with_price,
        })
    }

    pub fn check_account_health(&self, requirement_type: RiskRequirementType) -> MarginfiResult {
        let (total_weighted_assets, total_weighted_liabilities) = self
            .bank_accounts_with_price
            .iter()
            .map(|a| a.get_weighted_assets_and_liabilities(requirement_type.to_weight_type()))
            .try_fold((I80F48::ZERO, I80F48::ZERO), |(ta, tl), res| {
                let (assets, liabilities) = res?;
                let total_assets_sum = ta.checked_add(assets).ok_or_else(math_error!())?;
                let total_liabilities_sum =
                    tl.checked_add(liabilities).ok_or_else(math_error!())?;

                Ok::<_, ProgramError>((total_assets_sum, total_liabilities_sum))
            })?;

        check!(
            total_weighted_assets > total_weighted_liabilities,
            MarginfiError::BadAccountHealth
        );

        Ok(())
    }
}

const MAX_LENDING_ACCOUNT_BALANCES: usize = 16;

#[zero_copy]
pub struct LendingAccount {
    pub balances: [Option<Balance>; MAX_LENDING_ACCOUNT_BALANCES],
}

impl LendingAccount {
    pub fn get_first_empty_balance(&self) -> Option<usize> {
        self.balances.iter().position(|b| b.is_none())
    }

    pub fn get_active_balances_iter(&self) -> impl Iterator<Item = &Balance> {
        self.balances.iter().filter_map(|b| b.as_ref())
    }
}

#[zero_copy]
pub struct Balance {
    pub bank_index: u8,
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

enum TransferType {
    Deposit,
    Withdraw,
}

pub struct BankAccountWrapper<'a> {
    pub balance: &'a mut Balance,
    pub bank: &'a mut Bank,
}

impl<'a> BankAccountWrapper<'a> {
    pub fn find_by_mint_or_create<'b>(
        mint: Pubkey,
        lending_pool: &'a mut LendingPool,
        lending_account: &'a mut LendingAccount,
    ) -> MarginfiResult<BankAccountWrapper<'a>> {
        let (bank_index, bank) = lending_pool
            .banks
            .iter_mut()
            .enumerate()
            .find(|(_, b)| b.mint == mint)
            .ok_or_else(|| error!(MarginfiError::BankNotFound))?;

        let balance_index = lending_account
            .get_active_balances_iter()
            .position(|b| b.bank_index as usize == bank_index);

        let balance = if let Some(index) = balance_index {
            lending_account
                .balances
                .get_mut(index)
                .ok_or_else(|| error!(MarginfiError::LendingAccountBalanceNotFound))?
        } else {
            let empty_index = lending_account
                .get_first_empty_balance()
                .ok_or_else(|| error!(MarginfiError::LendingAccountBalanceSlotsFull))?;

            lending_account.balances[empty_index] = Some(Balance {
                bank_index: bank_index as u8,
                deposit_shares: I80F48::ZERO.into(),
                liability_shares: I80F48::ZERO.into(),
            });

            lending_account.balances.get_mut(empty_index).unwrap()
        }
        .as_mut()
        .unwrap();

        Ok(Self { balance, bank })
    }

    pub fn deposit(&mut self, amount: u64) -> MarginfiResult {
        let balance = &mut self.balance;
        let bank = &mut self.bank;

        let amount = I80F48::from_num(amount);

        let liability_shares: I80F48 = balance.liability_shares.into();

        let liability_value = bank.get_liability_value(liability_shares)?;

        let (deposit_value_delta, liability_replay_value_delta) = (
            max(
                amount
                    .checked_sub(liability_value)
                    .ok_or_else(math_error!())?,
                I80F48::ZERO,
            ),
            min(liability_value, amount),
        );

        let deposit_shares_delta = bank.get_deposit_shares(deposit_value_delta)?;
        balance.change_deposit_shares(deposit_shares_delta)?;
        bank.change_deposit_shares(deposit_shares_delta)?;

        let liability_shares_delta = bank.get_liability_shares(liability_replay_value_delta)?;
        balance.change_liability_shares(-liability_shares_delta)?;
        bank.change_liability_shares(-liability_shares_delta)?;

        Ok(())
    }

    pub fn withdraw(&mut self, amount: u64) -> MarginfiResult {
        let balance = &mut self.balance;
        let bank = &mut self.bank;

        let amount = I80F48::from_num(amount);

        let deposit_shares: I80F48 = balance.deposit_shares.into();

        let deposit_value = bank.get_deposit_value(deposit_shares)?;

        let (deposit_remove_value_delta, liability_value_delta) = (
            min(deposit_value, amount),
            max(
                amount
                    .checked_sub(deposit_value)
                    .ok_or_else(math_error!())?,
                I80F48::ZERO,
            ),
        );

        let deposit_shares_delta = bank.get_deposit_shares(deposit_remove_value_delta)?;
        balance.change_deposit_shares(-deposit_shares_delta)?;
        bank.change_deposit_shares(-deposit_shares_delta)?;

        let liability_shares_delta = bank.get_liability_shares(liability_value_delta)?;
        balance.change_liability_shares(liability_shares_delta)?;
        bank.change_liability_shares(liability_shares_delta)?;

        Ok(())
    }

    pub fn deposit_transfer<'b: 'c, 'c: 'b>(
        &self,
        amount: u64,
        accounts: Transfer<'b>,
        program: AccountInfo<'c>,
    ) -> MarginfiResult {
        check!(
            accounts.to.key.eq(&self.bank.liquidity_vault),
            MarginfiError::InvalidTransfer
        );

        transfer(CpiContext::new(program, accounts), amount)
    }

    pub fn withdraw_transfer<'b: 'c, 'c: 'b>(
        &self,
        amount: u64,
        accounts: Transfer<'b>,
        program: AccountInfo<'c>,
    ) -> MarginfiResult {
        check!(
            accounts.from.key.eq(&self.bank.liquidity_vault),
            MarginfiError::InvalidTransfer
        );

        transfer(CpiContext::new(program, accounts), amount)
    }
}
