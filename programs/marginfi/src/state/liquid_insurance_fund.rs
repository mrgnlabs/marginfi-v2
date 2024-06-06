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
    pub vault_authority: Pubkey,
    pub min_withdraw_period: i64,
    pub last_update: i64,

    pub total_shares: WrappedI80F48,
    pub share_value: WrappedI80F48,

    pub lif_vault_bump: u8,
    pub lif_authority_bump: u8,

    // TODO
    pub _padding: [[u64; 2]; 28],
}

impl LiquidInsuranceFund {
    pub fn initialize(
        &mut self,
        bank: Pubkey,
        vault_authority: Pubkey,
        min_withdraw_period: i64,
        lif_vault_bump: u8,
        lif_authority_bump: u8,
    ) {
        *self = LiquidInsuranceFund {
            bank,
            min_withdraw_period,
            total_shares: I80F48::ZERO.into(),
            share_value: I80F48::ONE.into(),
            last_update: i64::MIN,
            vault_authority,
            lif_vault_bump,
            lif_authority_bump,
            _padding: [[0; 2]; 28],
        };
    }

    pub fn process_withdrawal(
        &mut self,
        withdrawal: &mut LiquidInsuranceFundWithdrawal,
    ) -> MarginfiResult<u64> {
        // Fetch shares to withdraw
        let withdraw_shares: I80F48 = withdrawal.amount.into();

        // Calculate token amount
        let withdraw_token_amount = self.get_value(withdraw_shares)?;

        // Decrement total share count and update share value
        self.withdraw_shares(withdraw_shares, withdraw_token_amount);

        Ok(withdraw_token_amount.to_num::<u64>())
    }

    pub(crate) fn deposit_spl_transfer<'b: 'c, 'c: 'b>(
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

    pub(crate) fn withdraw_spl_transfer<'b: 'c, 'c: 'b>(
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

    pub(crate) fn deposit_shares(
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

    pub(crate) fn withdraw_shares(
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
    pub(crate) fn increase_balance_internal(&mut self, shares: I80F48) -> MarginfiResult {
        check!(shares > I80F48::ZERO, MarginfiError::InvalidTransfer);

        // Add new shares to existing collection of shares
        self.add_shares(shares)?;

        Ok(())
    }

    pub(crate) fn update_share_price_internal(
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

    pub(crate) fn add_shares(&mut self, shares: I80F48) -> MarginfiResult {
        let total_shares: I80F48 = self.total_shares.into();
        self.total_shares = total_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();
        Ok(())
    }

    pub(crate) fn remove_shares(&mut self, shares: I80F48) -> MarginfiResult {
        let total_shares: I80F48 = self.total_shares.into();
        self.total_shares = total_shares
            .checked_sub(shares)
            .ok_or_else(math_error!())?
            .into();
        Ok(())
    }

    pub(crate) fn get_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.share_value.into())
            .ok_or_else(math_error!())?)
    }

    /// Internal arithmetic for decreasing the balance of the liquid insurance fund
    pub(crate) fn decrease_balance_internal(&mut self, amount: I80F48) -> MarginfiResult {
        check!(amount > I80F48::ZERO, MarginfiError::InvalidTransfer);

        let current_amount = self.get_value(self.total_shares.into())?;
        let delta_decrease = min(current_amount, amount);

        let share_decrease = self.get_shares(delta_decrease)?;

        // Remove shares from existing collection of shares
        self.remove_shares(share_decrease)?;

        Ok(())
    }

    pub(crate) fn get_value(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub(crate) fn remove_shares(&mut self, delta: I80F48) -> MarginfiResult {
        let total_shares: I80F48 = self.total_shares.into();
        let new_shares = total_shares.checked_sub(delta).ok_or_else(math_error!())?;
        check!(new_shares >= 0, MarginfiError::MathError);
        self.total_shares = new_shares.into();
        Ok(())
    }

    pub(crate) fn haircut_shares(&mut self, decrease_amount: u64) -> MarginfiResult {
        let total_shares: I80F48 = self.total_shares.into();
        let share_value: I80F48 = self.share_value.into();

        let new_share_value = total_shares
            .checked_mul(share_value)
            .ok_or_else(math_error!())?
            .checked_sub(decrease_amount.into())
            .ok_or_else(math_error!())?
            .checked_div(total_shares)
            .ok_or_else(math_error!())?;

        self.share_value = new_share_value.into();

        Ok(())
    }
}

#[account(zero_copy)]
pub struct LiquidInsuranceFundAccount {
    pub authority: Pubkey,
    pub balances: [LiquidInsuranceFundBalance; 16],
    pub withdrawals: [LiquidInsuranceFundWithdrawal; 16],
    pub padding: [u64; 8],
}

impl LiquidInsuranceFundAccount {
    pub fn initialize(&mut self, authority: Pubkey) {
        self.authority = authority;
    }

    /// Try to find an existing deposit for this insurance vault,
    /// fallback to a new deposit if possible
    pub fn get_or_init_deposit(
        &mut self,
        bank_insurance_fund: &Pubkey,
    ) -> Option<&mut LiquidInsuranceFundBalance> {
        let mut maybe_new = None;
        for balance in self.balances.iter_mut() {
            if balance.bank_insurance_fund.eq(bank_insurance_fund) {
                return Some(balance);
            } else if balance.is_empty() {
                maybe_new.get_or_insert(balance);
            }
        }

        maybe_new.map(|deposit| {
            deposit.bank_insurance_fund = *bank_insurance_fund;
            deposit
        })
    }

    /// Try to find an existing withdraw claim for this insurance vault.
    /// If there are multiple, the oldest/earliest is returned
    pub fn get_earliest_withdrawal(
        &mut self,
        bank_insurance_fund: &Pubkey,
    ) -> Option<&mut LiquidInsuranceFundWithdrawal> {
        let mut oldest_withdrawal = None;
        for (i, withdrawal) in self.withdrawals.iter().enumerate() {
            if withdrawal.bank_insurance_fund.eq(bank_insurance_fund) {
                if self.withdrawals[*oldest_withdrawal.get_or_insert(i)].withdraw_request_timestamp
                    > withdrawal.withdraw_request_timestamp
                {
                    oldest_withdrawal.replace(i);
                }
            }
        }

        oldest_withdrawal.map(|i| &mut self.withdrawals[i])
    }

    /// 1) Look for existing deposit
    /// 2) Validate and decrement requested shares
    /// 3) Initialize withdrawal in empty slot
    pub fn create_withdrawal(
        &mut self,
        bank_insurance_fund: &Pubkey,
        // All if None
        shares: Option<I80F48>,
        timestamp: i64,
    ) -> MarginfiResult<I80F48> {
        // 1) Look for existing deposit
        let deposit = self
            .balances
            .iter_mut()
            .find(|deposit| deposit.bank_insurance_fund.eq(bank_insurance_fund))
            .ok_or(MarginfiError::InvalidWithdrawal)?;

        // 2) Validate and decrement requested shares
        let deposit_shares: I80F48 = deposit.shares.into();
        let requested_shares = shares.unwrap_or(deposit_shares);
        if requested_shares > deposit_shares {
            return err!(MarginfiError::InvalidWithdrawal);
        }
        deposit.shares = (deposit_shares - requested_shares).into();

        // 3) Initialize withdrawal in empty slot
        let withdrawal_slot = self
            .withdrawals
            .iter_mut()
            .find(|slot| slot.is_empty())
            .ok_or(MarginfiError::InsuranceFundAccountWithdrawSlotsFull)?;
        *withdrawal_slot = LiquidInsuranceFundWithdrawal {
            bank_insurance_fund: *bank_insurance_fund,
            amount: requested_shares.into(),
            withdraw_request_timestamp: timestamp,
        };

        Ok(requested_shares)
    }
}

#[account(zero_copy)]
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct LiquidInsuranceFundAccountData {
    pub balances: [LiquidInsuranceFundBalance; 16],
    pub withdrawals: [LiquidInsuranceFundWithdrawal; 16],
}

#[account(zero_copy)]
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct LiquidInsuranceFundBalance {
    pub bank_insurance_fund: Pubkey,
    pub shares: WrappedI80F48,
}

impl LiquidInsuranceFundBalance {
    fn is_empty(&self) -> bool {
        self.shares.value == I80F48::ZERO.to_le_bytes()
    }

    /// Can be used to subtract given I80F48 is signed
    pub fn add_shares(&mut self, shares: I80F48) {
        let current_shares: I80F48 = self.shares.into();
        self.shares = (current_shares + shares).into();
    }
}

#[account(zero_copy)]
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct LiquidInsuranceFundWithdrawal {
    pub bank_insurance_fund: Pubkey,
    pub amount: WrappedI80F48,
    pub withdraw_request_timestamp: i64,
}

impl LiquidInsuranceFundWithdrawal {
    pub fn is_empty(&self) -> bool {
        self.amount.value == I80F48::ZERO.to_le_bytes()
    }

    pub fn shares(&self) -> I80F48 {
        self.amount.into()
    }

    pub fn free(&mut self) {
        *self = LiquidInsuranceFundWithdrawal {
            bank_insurance_fund: Pubkey::default(),
            amount: I80F48::ZERO.into(),
            withdraw_request_timestamp: i64::MAX,
        }
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
