use std::cmp::min;

use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Transfer};
use fixed::types::I80F48;

use crate::{check, debug, math_error, MarginfiError, MarginfiResult};

use super::marginfi_group::WrappedI80F48;

/// Fund that represents tokenized insurance pool backed by liquidators
/// Uses the bank's underlying insurance vault to deposit funds
/// Creates a mint representing the tokens managed by the lif
#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct LiquidInsuranceFund {
    pub bank: Pubkey,

    pub min_withdraw_period: i64,

    pub last_update: i64,

    // Share/Deposit model of the underlying insurance vault
    pub total_shares: WrappedI80F48,
    pub share_value: WrappedI80F48,

    pub liquid_insurance_bump: u8,

    // TODO
    pub _padding: [[u64; 2]; 28],
}

impl LiquidInsuranceFund {
    pub fn new(
        bank: Pubkey,
        min_withdraw_period: i64,
        current_timestamp: i64,
        liquid_insurance_bump: u8,
    ) -> MarginfiResult<Self> {
        Ok(LiquidInsuranceFund {
            bank,

            min_withdraw_period,

            total_shares: I80F48::ZERO.into(),
            share_value: I80F48::ONE.into(),

            last_update: current_timestamp,

            liquid_insurance_bump,

            _padding: [[0; 2]; 28],
        })
    }

    pub fn deposit_spl_transfer<'b: 'c, 'c: 'b>(
        &self,
        amount: u64,
        accounts: Transfer<'b>,
        program: AccountInfo<'c>,
    ) -> MarginfiResult {
        // Only deposits to the bank's insurance vault are allowed.
        // TODO add check against bank insurance vault? By deriving address

        debug!(
            "deposit_spl_transfer: amount: {} from {} to {}, auth {}",
            amount, accounts.from.key, accounts.to.key, accounts.authority.key
        );

        transfer(CpiContext::new(program, accounts), amount)
    }

    pub fn withdraw_spl_transfer<'b: 'c, 'c: 'b>(
        &self,
        amount: u64,
        accounts: Transfer<'b>,
        program: AccountInfo<'c>,
    ) -> MarginfiResult {
        // Only withdraws from the bank's insurance vault are allowed.
        // TODO add check against bank insurance vault? By deriving address

        debug!(
            "withdraw_spl_transfer: amount: {} from {} to {}, auth {}",
            amount, accounts.from.key, accounts.to.key, accounts.authority.key
        );

        transfer(CpiContext::new(program, accounts), amount)
    }

    pub fn deposit_shares(
        &mut self,
        shares: I80F48,
        bank_insurance_vault_amount: I80F48,
    ) -> MarginfiResult {
        // Update the internal count of shares
        self.increase_balance_internal(shares)?;

        // Update share price
        self.update_share_price_internal(bank_insurance_vault_amount)?;

        Ok(())
    }

    pub fn withdraw_shares(
        &mut self,
        amount: I80F48,
        bank_insurance_vault_amount: I80F48,
    ) -> MarginfiResult {
        // Update the internal count of shares
        self.decrease_balance_internal(amount)?;

        // Update the share price
        self.update_share_price_internal(bank_insurance_vault_amount)?;

        Ok(())
    }

    /// Internal arithmetic for increase the balance of the liquid insurance fund
    pub fn increase_balance_internal(&mut self, shares: I80F48) -> MarginfiResult {
        check!(shares > I80F48::ZERO, MarginfiError::InvalidTransfer);

        // Add new shares to existing collection of shares
        self.add_shares(shares)?;

        Ok(())
    }

    pub fn update_share_price_internal(
        &mut self,
        bank_insurance_vault_amount: I80F48,
    ) -> MarginfiResult {
        // Update share price based on latest number of shares and the
        // number of shares in the bank insurance vault
        self.share_value = bank_insurance_vault_amount
            .checked_div(self.total_shares.into())
            .ok_or_else(math_error!())?
            .into();

        Ok(())
    }

    pub fn add_shares(&mut self, shares: I80F48) -> MarginfiResult {
        let total_shares: I80F48 = self.total_shares.into();
        self.total_shares = total_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();
        Ok(())
    }

    pub fn get_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.share_value.into())
            .ok_or_else(math_error!())?)
    }

    /// Internal arithmetic for decreasing the balance of the liquid insurance fund
    pub fn decrease_balance_internal(&mut self, amount: I80F48) -> MarginfiResult {
        check!(amount > I80F48::ZERO, MarginfiError::InvalidTransfer);

        let current_shares: I80F48 = self.total_shares.into();
        let current_amount = self.get_value(current_shares.into())?;
        let delta_decrease = min(current_amount, amount);

        let share_decrease = self.get_shares(delta_decrease)?;

        // Remove shares from existing collection of shares
        self.remove_shares(share_decrease)?;

        Ok(())
    }

    pub fn get_value(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn remove_shares(&mut self, delta: I80F48) -> MarginfiResult {
        let total_shares: I80F48 = self.total_shares.into();
        self.total_shares = total_shares
            .checked_sub(delta)
            .ok_or_else(math_error!())?
            .into();
        Ok(())
    }
}

#[test]
fn test_share_deposit_accounting() {
    let mut lif = LiquidInsuranceFund::new();

    // Total bank vault amount = 1_000_000
    // Total number of shares = 10 by default
    // Price per share initial = 1000
    assert!(lif.total_shares == I80F48::from_num(10).into());
    assert!(lif.share_value == I80F48::from(100_000).into());

    let user_deposit_amount = I80F48::from_num(100_000);
    let bank_insurance_vault_amount_first_deposit = I80F48::from_num(1_000_000);

    // User deposits 10 units of tokens
    lif.deposit_shares(
        user_deposit_amount,
        bank_insurance_vault_amount_first_deposit,
    )
    .unwrap();

    assert!(lif.total_shares == I80F48::from(11).into()); // 11
    assert!(
        lif.share_value
            == bank_insurance_vault_amount_first_deposit
                .checked_div(I80F48::from_num(11))
                .unwrap()
                .into()
    ); // 90909.09090909090909

    // Deposit some more shares
    let user_deposit_amount_2 = I80F48::from_num(1_000);
    let bank_insurance_vault_amount_second_deposit = I80F48::from_num(1_000_100);
    lif.deposit_shares(
        user_deposit_amount_2,
        bank_insurance_vault_amount_second_deposit,
    )
    .unwrap();

    println!("{:?}", lif.total_shares);
    println!("{:?}", lif.share_value);
}

#[test]
fn test_share_withdraw_accounting() {
    let mut lif = LiquidInsuranceFund::new();

    // Total bank vault amount = 1_000_000
    // Total number of shares = 10 by default
    // Price per share initial = 1000
    assert!(lif.total_shares == I80F48::from_num(10).into());
    assert!(lif.share_value == I80F48::from(100_000).into());

    // uses shares instead of amounts (representing units of the liquid token)
    let user_withdraw_shares = I80F48::from_num(5);
    let bank_insurance_vault_amount_first_withdrawal = I80F48::from_num(1_000_000);

    lif.withdraw_shares(
        user_withdraw_shares,
        bank_insurance_vault_amount_first_withdrawal,
    )
    .unwrap();

    assert!(lif.total_shares == I80F48::from(5).into()); // 5
    assert!(lif.share_value == I80F48::from_num(200_000).into()); // 200k
}
