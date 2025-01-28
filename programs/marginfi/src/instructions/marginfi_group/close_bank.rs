use anchor_lang::prelude::*;

use crate::{
    check,
    state::marginfi_group::{Bank, MarginfiGroup},
    MarginfiError, MarginfiResult,
};

pub fn close_bank(ctx: Context<BankClose>) -> MarginfiResult {
    let bank = &ctx.accounts.bank.load()?;

    check!(
        bank.can_be_closed(),
        MarginfiError::IllegalAction,
        "Bank cannot be closed"
    );

    Ok(())
}

#[derive(Accounts)]
pub struct BankClose<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(mut, constraint = bank.load()?.group == marginfi_group.key(), close = fee_payer)]
    pub bank: AccountLoader<'info, Bank>,
    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
}
