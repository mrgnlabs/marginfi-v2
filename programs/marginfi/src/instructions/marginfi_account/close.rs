use anchor_lang::prelude::*;

use crate::{check, state::marginfi_account::MarginfiAccount, MarginfiError, MarginfiResult};

pub fn close_account(ctx: Context<MarginfiAccountClose>) -> MarginfiResult {
    let marginfi_account = &ctx.accounts.marginfi_account.load()?;

    check!(
        marginfi_account.can_be_closed(),
        MarginfiError::IllegalAction,
        "Account cannot be closed"
    );

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiAccountClose<'info> {
    #[account(mut, close = fee_payer)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(address = marginfi_account.load()?.authority)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
}
