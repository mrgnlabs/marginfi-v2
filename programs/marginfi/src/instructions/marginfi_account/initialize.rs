use crate::{
    events::{AccountEventHeader, MarginfiAccountCreateEvent},
    prelude::*,
    state::marginfi_account::MarginfiAccount,
};
use anchor_lang::prelude::*;
use solana_program::sysvar::Sysvar;

pub fn initialize_account(ctx: Context<MarginfiAccountInitialize>) -> MarginfiResult {
    let MarginfiAccountInitialize {
        authority,
        marginfi_group,
        marginfi_account: marginfi_account_loader,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account_loader.load_init()?;

    marginfi_account.initialize(marginfi_group.key(), authority.key());

    emit!(MarginfiAccountCreateEvent {
        header: AccountEventHeader {
            signer: Some(authority.key()),
            marginfi_account: marginfi_account_loader.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        }
    });

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
