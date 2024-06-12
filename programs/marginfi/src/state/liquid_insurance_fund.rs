use std::cmp::min;

use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Transfer};
use fixed::types::I80F48;

use crate::{
    check,
    constants::{LIQUID_INSURANCE_SEED, LIQUID_INSURANCE_USER_SEED},
    debug, math_error, MarginfiError, MarginfiResult,
};

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
    /// This value is only updated at the beginning of relevant LIF instructions and
    /// may be outdated. For the most up-to-date value (relevant for off-chain requests),
    /// take the insurance vault balance and divide it by the total number of shares.
    pub lazy_share_value: WrappedI80F48,
    pub admin_shares: WrappedI80F48,

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
        balance: u64,
    ) {
        let admin_shares = I80F48::from(balance);
        *self = LiquidInsuranceFund {
            bank,
            min_withdraw_period,
            total_shares: admin_shares.into(),
            lazy_share_value: I80F48::ONE.into(),
            admin_shares: self.admin_shares.into(),
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
        let withdraw_shares: I80F48 = withdrawal.shares.into();

        // Calculate token amount
        let withdraw_token_amount = self.get_value(withdraw_shares)?.to_num();

        // Decrement total share count and update share value
        self.withdraw_shares(withdraw_shares)?;

        // Free withdrawal slot
        withdrawal.free();

        Ok(withdraw_token_amount)
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
        signer_seeds: &[&[&[u8]]],
    ) -> MarginfiResult {
        // Only withdraws from the bank's insurance vault are allowed.
        // TODO add check against bank insurance vault? By deriving address

        debug!(
            "withdraw_spl_transfer: amount: {} from {} to {}, auth {}",
            amount, accounts.from.key, accounts.to.key, accounts.authority.key
        );

        transfer(
            CpiContext::new_with_signer(program, accounts, signer_seeds),
            amount,
        )
    }

    pub(crate) fn deposit_shares(&mut self, shares: I80F48) -> MarginfiResult {
        // Update the internal count of shares
        self.increase_balance_internal(shares)?;

        Ok(())
    }

    pub(crate) fn withdraw_shares(&mut self, amount: I80F48) -> MarginfiResult {
        // Update the internal count of shares
        self.decrease_balance_internal(amount)?;

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
        bank_insurance_vault_amount: u64,
    ) -> MarginfiResult {
        // Reset share value if there are no shares
        if self.get_total_shares() == I80F48::ZERO {
            self.lazy_share_value = I80F48::ONE.into();
            return Ok(());
        }

        // Update share price based on latest number of shares and the
        // number of shares in the bank insurance vault
        self.lazy_share_value = I80F48::from(bank_insurance_vault_amount)
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

    pub(crate) fn get_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.lazy_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn get_admin_shares(&self) -> I80F48 {
        self.admin_shares.into()
    }

    pub fn get_total_shares(&self) -> I80F48 {
        self.total_shares.into()
    }

    pub(crate) fn set_admin_shares(&mut self, shares: I80F48) {
        self.admin_shares = shares.into();
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
            .checked_mul(self.lazy_share_value.into())
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
        let share_value: I80F48 = self.lazy_share_value.into();

        let new_share_value = total_shares
            .checked_mul(share_value)
            .ok_or_else(math_error!())?
            .checked_sub(decrease_amount.into())
            .ok_or_else(math_error!())?
            .checked_div(total_shares)
            .ok_or_else(math_error!())?;

        self.lazy_share_value = new_share_value.into();

        Ok(())
    }

    /// Utility function for attempting to load a lif; a bank may or may not have a lif.
    ///
    /// This does NOT check the account address. Caller must do this!
    ///
    /// Assuming address is checked:
    /// 1) If the bank has a lif, this will return Ok(lif).
    /// 2) If the bank does not have a lif, Err(e) will be returned.
    ///
    /// NOTE: this is done because if AccountLoader<'info, T> is used in a #[derive(Accounts)]
    /// struct in case 2), the instruction will simply fail due to an account owner check.
    pub fn try_loader<'a, 'info>(
        ai: &'a AccountInfo<'info>,
        // ) -> MarginfiResult<AccountLoader<'info, LiquidInsuranceFund>> {
    ) -> MarginfiResult<AccountLoader<'info, LiquidInsuranceFund>> {
        AccountLoader::try_from_unchecked(&crate::ID, ai)
    }

    pub(crate) fn maybe_process_admin_withdraw(
        liquid_insurance_fund: &AccountInfo,
        insurance_vault_balance: u64,
        amount: I80F48,
    ) -> MarginfiResult<u64> {
        let tokens = if let Ok(lif) = LiquidInsuranceFund::try_loader(&liquid_insurance_fund) {
            let mut lif = lif.load_mut()?;
            lif.update_share_price_internal(insurance_vault_balance)?;

            let admin_shares = lif.get_admin_shares();
            if admin_shares < amount {
                return err!(MarginfiError::InvalidWithdrawal);
            }

            let token_amount = lif.get_value(amount)?;
            lif.remove_shares(amount)?;
            lif.set_admin_shares(admin_shares - amount);

            token_amount.to_num::<u64>()
        } else {
            amount.to_num::<u64>()
        };

        Ok(tokens)
    }

    /// This does NOT check the lif account address. Caller must do this!
    pub(crate) fn maybe_process_admin_deposit(
        liquid_insurance_fund: &AccountInfo,
        insurance_vault_balance: u64,
        amount: u64,
    ) -> MarginfiResult<()> {
        if let Ok(lif) = LiquidInsuranceFund::try_loader(liquid_insurance_fund) {
            let mut lif = lif.load_mut()?;
            lif.update_share_price_internal(insurance_vault_balance)?;

            let admin_shares = lif.get_admin_shares();
            let added_shares = lif.get_shares(amount.into())?;
            let new_admin_shares = admin_shares + added_shares;
            lif.set_admin_shares(new_admin_shares);
            lif.add_shares(added_shares)?;
        }

        Ok(())
    }

    pub fn maybe_process_bankruptcy(
        liquid_insurance_fund: &AccountInfo,
        amount_covered_by_insurance: I80F48,
    ) -> MarginfiResult<()> {
        if let Ok(lif) = LiquidInsuranceFund::try_loader(liquid_insurance_fund) {
            let amount_covered_by_insurance = amount_covered_by_insurance
                .checked_to_num::<u64>()
                .ok_or(MarginfiError::MathError)?;
            let mut lif = lif.load_mut()?;
            lif.haircut_shares(amount_covered_by_insurance)?;
        }

        Ok(())
    }

    pub fn address(bank: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[LIQUID_INSURANCE_SEED.as_bytes(), bank.as_ref()],
            &crate::ID,
        )
        .0
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
    pub fn address(user: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[LIQUID_INSURANCE_USER_SEED.as_bytes(), user.as_ref()],
            &crate::ID,
        )
        .0
    }

    pub fn initialize(&mut self, authority: Pubkey) {
        self.authority = authority;
    }

    pub fn get_deposit(
        &mut self,
        bank_insurance_fund: &Pubkey,
    ) -> Option<&LiquidInsuranceFundBalance> {
        for balance in self.balances.iter() {
            if balance.bank_insurance_fund.eq(bank_insurance_fund) {
                return Some(balance);
            }
        }
        None
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
        let new_shares = deposit_shares - requested_shares;
        deposit.shares = new_shares.into();
        if new_shares == I80F48::ZERO {
            deposit.free();
        }

        // 3) Initialize withdrawal in empty slot
        let withdrawal_slot = self
            .withdrawals
            .iter_mut()
            .find(|slot| slot.is_empty())
            .ok_or(MarginfiError::InsuranceFundAccountWithdrawSlotsFull)?;
        *withdrawal_slot = LiquidInsuranceFundWithdrawal {
            bank_insurance_fund: *bank_insurance_fund,
            shares: requested_shares.into(),
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
#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Default)]
pub struct LiquidInsuranceFundBalance {
    pub bank_insurance_fund: Pubkey,
    pub shares: WrappedI80F48,
}

impl LiquidInsuranceFundBalance {
    fn is_empty(&self) -> bool {
        self.shares.value == I80F48::ZERO.to_le_bytes()
    }

    pub fn shares(&self) -> I80F48 {
        self.shares.into()
    }

    /// Can be used to subtract given I80F48 is signed
    pub fn add_shares(&mut self, shares: I80F48) {
        let current_shares: I80F48 = self.shares.into();
        self.shares = (current_shares + shares).into();
    }

    pub(crate) fn free(&mut self) {
        *self = Default::default();
    }
}

#[account(zero_copy)]
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Default, PartialEq)]
pub struct LiquidInsuranceFundWithdrawal {
    pub bank_insurance_fund: Pubkey,
    pub shares: WrappedI80F48,
    pub withdraw_request_timestamp: i64,
}

impl LiquidInsuranceFundWithdrawal {
    pub fn is_empty(&self) -> bool {
        self.shares.value == I80F48::ZERO.to_le_bytes()
    }

    pub fn shares(&self) -> I80F48 {
        self.shares.into()
    }

    pub fn free(&mut self) {
        *self = Default::default();
    }
}

// #[test]
// fn test_share_deposit_accounting() {
//     let mut lif = LiquidInsuranceFund::new();

//     // Total bank vault amount = 1_000_000
//     // Total number of shares = 10 by default
//     // Price per share initial = 1000
//     assert!(lif.total_shares == I80F48::from_num(10).into());
//     assert!(lif.share_value == I80F48::from(100_000).into());

//     let user_deposit_amount = I80F48::from_num(100_000);
//     let bank_insurance_vault_amount_first_deposit = I80F48::from_num(1_000_000);

//     // User deposits 10 units of tokens
//     lif.deposit_shares(
//         user_deposit_amount,
//         bank_insurance_vault_amount_first_deposit,
//     )
//     .unwrap();

//     assert!(lif.total_shares == I80F48::from(11).into()); // 11
//     assert!(
//         lif.share_value
//             == bank_insurance_vault_amount_first_deposit
//                 .checked_div(I80F48::from_num(11))
//                 .unwrap()
//                 .into()
//     ); // 90909.09090909090909

//     // Deposit some more shares
//     let user_deposit_amount_2 = I80F48::from_num(1_000);
//     let bank_insurance_vault_amount_second_deposit = I80F48::from_num(1_000_100);
//     lif.deposit_shares(
//         user_deposit_amount_2,
//         bank_insurance_vault_amount_second_deposit,
//     )
//     .unwrap();

//     println!("{:?}", lif.total_shares);
//     println!("{:?}", lif.share_value);
// }

// #[test]
// fn test_share_withdraw_accounting() {
//     let mut lif = LiquidInsuranceFund::new();

//     // Total bank vault amount = 1_000_000
//     // Total number of shares = 10 by default
//     // Price per share initial = 1000
//     assert!(lif.total_shares == I80F48::from_num(10).into());
//     assert!(lif.share_value == I80F48::from(100_000).into());

//     // uses shares instead of amounts (representing units of the liquid token)
//     let user_withdraw_shares = I80F48::from_num(5);
//     let bank_insurance_vault_amount_first_withdrawal = I80F48::from_num(1_000_000);

//     lif.withdraw_shares(
//         user_withdraw_shares,
//         bank_insurance_vault_amount_first_withdrawal,
//     )
//     .unwrap();

//     assert!(lif.total_shares == I80F48::from(5).into()); // 5
//     assert!(lif.share_value == I80F48::from_num(200_000).into()); // 200k
// }
