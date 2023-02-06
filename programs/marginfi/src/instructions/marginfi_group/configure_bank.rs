use crate::{
    state::marginfi_group::{Bank, BankConfigOpt, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;

pub fn lending_pool_configure_bank(
    ctx: Context<LendingPoolConfigureBank>,
    bank_config: BankConfigOpt,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    bank.configure(&bank_config)?;

    if bank_config.oracle.is_some() {
        bank.config.validate_oracle_setup(ctx.remaining_accounts)?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBank<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,
}
