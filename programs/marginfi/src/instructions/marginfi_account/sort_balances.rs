use anchor_lang::prelude::*;

use crate::{state::marginfi_account::MarginfiAccount, MarginfiResult};

pub fn lending_account_sort_balances<'info>(
    ctx: Context<'_, '_, 'info, 'info, SortBalances<'info>>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    marginfi_account.lending_account.sort_balances();
    Ok(())
}

#[derive(Accounts)]
pub struct SortBalances<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
}
