use crate::utils::NumTraitsWithTolerance;
use crate::{
    check,
    constants::{CLOSE_ENABLED_FLAG, ZERO_AMOUNT_THRESHOLD},
    state::marginfi_group::{Bank, MarginfiGroup},
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

pub fn lending_pool_close_bank(ctx: Context<LendingPoolCloseBank>) -> MarginfiResult {
    let bank = ctx.accounts.bank.load()?;

    check!(
        bank.get_flag(CLOSE_ENABLED_FLAG),
        MarginfiError::IllegalAction
    );
    check!(
        bank.lending_position_count == 0
            && bank.borrowing_position_count == 0
            && bank.position_count == 0,
        MarginfiError::IllegalAction
    );
    check!(
        I80F48::from(bank.total_asset_shares).is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD)
            && I80F48::from(bank.total_liability_shares)
                .is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
        MarginfiError::IllegalAction
    );
    check!(
        I80F48::from(bank.emissions_remaining).is_zero_with_tolerance(I80F48::ZERO),
        MarginfiError::IllegalAction
    );

    drop(bank);

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolCloseBank<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,
    #[account(
        mut,
        has_one = group,
        close = admin
    )]
    pub bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub admin: Signer<'info>,
}
