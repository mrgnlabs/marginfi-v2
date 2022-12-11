use std::cmp::{max, min};

use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::{
    math_error,
    prelude::{MarginfiError, MarginfiResult},
};

use super::marginfi_group::{Bank, LendingPool, WrappedI80F48};

#[account(zero_copy)]
pub struct MarginfiAccount {
    pub group: Pubkey,
    pub owner: Pubkey,
}

impl MarginfiAccount {
    /// Set the initial data for the marginfi account.
    pub fn initialize(&mut self, group: Pubkey, owner: Pubkey) {
        self.owner = owner;
        self.group = group;
    }
}

const MAX_LENDING_ACCOUNT_BALANCES: usize = 16;

#[zero_copy]
pub struct LendingAccount {
    pub balances: [Balance; MAX_LENDING_ACCOUNT_BALANCES],
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

pub struct BankAccountWrapper<'a> {
    pub balance: &'a mut Balance,
    pub bank: &'a mut Bank,
}

impl<'a> BankAccountWrapper<'a> {
    pub fn create_account_wrapper(
        bank_index: u8,
        lending_pool: &'a mut LendingPool,
        lending_account: &'a mut LendingAccount,
    ) -> MarginfiResult<BankAccountWrapper<'a>> {
        let bank = lending_pool
            .banks
            .get_mut(bank_index as usize)
            .ok_or_else(|| error!(MarginfiError::LendingAccountBalanceNotFound))?;

        let balance = lending_account
            .balances
            .iter_mut()
            .find(|b| b.bank_index == bank_index)
            .ok_or_else(|| error!(MarginfiError::LendingAccountBalanceNotFound))?;

        Ok(Self { balance, bank })
    }

    pub fn deposit(&mut self, amount: u64) -> MarginfiResult {
        let balance = self.balance;
        let bank = self.bank;

        let amount = I80F48::from_num(amount);

        let deposit_shares: I80F48 = balance.deposit_shares.into();
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

    pub fn borrow(&mut self, amount: u64) -> MarginfiResult {
        let balance = self.balance;
        let bank = self.bank;

        let amount = I80F48::from_num(amount);

        let deposit_shares: I80F48 = balance.deposit_shares.into();
        let liability_shares: I80F48 = balance.liability_shares.into();

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
}
