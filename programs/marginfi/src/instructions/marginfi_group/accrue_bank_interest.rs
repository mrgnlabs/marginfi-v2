use crate::{
    state::marginfi_group::{Bank, MarginfiGroup},
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
        &*ctx.accounts.group.load()?,
        #[cfg(not(feature = "client"))]
        ctx.accounts.bank.key(),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolAccrueBankInterest<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        has_one = group
    )]
    pub bank: AccountLoader<'info, Bank>,
}
