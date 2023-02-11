use crate::prelude::*;
use crate::state::marginfi_account::MarginfiAccount;
use anchor_lang::prelude::*;
use solana_program::sysvar::Sysvar;

pub fn initialize(ctx: Context<MarginfiAccountInitialize>) -> MarginfiResult {
    let MarginfiAccountInitialize {
        authority,
        marginfi_group,
        marginfi_account,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account.load_init()?;

    marginfi_account.initialize(marginfi_group.key(), authority.key());

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiAccountInitialize<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + std::mem::size_of::<MarginfiAccount>()
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}