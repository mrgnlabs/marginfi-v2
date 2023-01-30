use crate::prelude::*;
use crate::state::marginfi_account::MarginfiAccount;
use anchor_lang::prelude::*;
use solana_program::sysvar::Sysvar;

pub fn initialize(ctx: Context<InitializeMarginfiAccount>) -> MarginfiResult {
    let InitializeMarginfiAccount {
        signer,
        marginfi_group,
        marginfi_account,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account.load_init()?;

    marginfi_account.initialize(marginfi_group.key(), signer.key());

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeMarginfiAccount<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        init,
        payer = signer,
        space = 8 + std::mem::size_of::<MarginfiAccount>()
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
