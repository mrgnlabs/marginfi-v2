use crate::{state::marginfi_group::MarginfiGroup, MarginfiResult};
use anchor_lang::prelude::*;

pub fn process(ctx: Context<InitializeMarginfiGroup>) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_init()?;

    let InitializeMarginfiGroup { admin, .. } = ctx.accounts;

    marginfi_group.set_initial_configuration(admin.key());

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeMarginfiGroup<'info> {
    #[account(zero)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}
