use crate::{
    state::marginfi_group::{GroupConfig, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;

/// Configure margin group
///
/// Admin only
pub fn configure(ctx: Context<MarginfiGroupConfigure>, config: GroupConfig) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_mut()?;

    marginfi_group.configure(config)?;

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiGroupConfigure<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
}
