use crate::check;
use crate::utils::NumTraitsWithTolerance;
use crate::{
    constants::{CLOSE_ENABLED_FLAG, ZERO_AMOUNT_THRESHOLD},
    state::marginfi_group::{Bank, MarginfiGroup},
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

pub fn lending_pool_close_bank(ctx: Context<LendingPoolCloseBank>) -> MarginfiResult {
    let mut group = ctx.accounts.group.load_mut()?;
    // Note: Groups created prior to 0.1.2 have a non-authoritative count here, so subtraction
    // without saturation could reduce the count below zero.
    group.banks = group.banks.saturating_sub(1);

    let bank = ctx.accounts.bank.load()?;

    // banks created prior to 0.1.4 can never be closed because we cannot guarantee an accurate
    // position count for those banks.
    check!(
        bank.get_flag(CLOSE_ENABLED_FLAG),
        MarginfiError::BankCannotClose,
        "Only banks created in 0.1.4 and later can close"
    );
    check!(
        bank.lending_position_count == 0 && bank.borrowing_position_count == 0,
        MarginfiError::BankCannotClose,
        "Only banks with no open positions can close"
    );
    check!(
        I80F48::from(bank.total_asset_shares).is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD)
            && I80F48::from(bank.total_liability_shares)
                .is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
        MarginfiError::BankCannotClose
    );
    check!(
        I80F48::from(bank.emissions_remaining).is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
        MarginfiError::BankCannotClose
    );

    drop(bank);

    // Bank will now be closed by anchor

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolCloseBank<'info> {
    #[account(
        mut,
        has_one = admin @ MarginfiError::InvalidAdminConstraint,
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroupConstraint,
        close = admin
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub admin: Signer<'info>,
}
