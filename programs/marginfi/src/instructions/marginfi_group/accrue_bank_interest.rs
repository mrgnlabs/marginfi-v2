use crate::{
    state::{bank::Bank, marginfi_group::MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;

pub fn lending_pool_accrue_bank_interest(
    ctx: Context<LendingPoolAccrueBankInterest>,
) -> MarginfiResult {
    let clock = Clock::get()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    bank.accrue_interest(
        clock.unix_timestamp,
        &*ctx.accounts.marginfi_group.load()?,
        #[cfg(not(feature = "client"))]
        ctx.accounts.bank.key(),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolAccrueBankInterest<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,
}
