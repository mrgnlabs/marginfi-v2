use crate::{
    events::{AccountEventHeader, MarginfiAccountTransferAccountAuthorityEvent},
    prelude::*,
    state::marginfi_account::MarginfiAccount,
};
use anchor_lang::prelude::*;

pub fn set_account_transfer_authority(
    ctx: Context<MarginfiAccountSetAccountAuthority>,
) -> MarginfiResult {
    // Gather accounts
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    let new_account_authority = ctx.accounts.new_authority.load()?;
    let signer = ctx.accounts.signer.key();

    // Gather authorities
    let new_authority = new_account_authority.authority;
    let old_authority = marginfi_account.authority;

    marginfi_account.set_new_account_authority_checked(new_authority)?;

    emit!(MarginfiAccountTransferAccountAuthorityEvent {
        header: AccountEventHeader {
            signer: Some(signer),
            marginfi_account: old_authority,
            marginfi_account_authority: new_authority,
            marginfi_group: marginfi_account.group,
        },
        old_account_authority: old_authority,
        new_account_authority: new_authority,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiAccountSetAccountAuthority<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        address = marginfi_account.load()?.authority,
    )]
    pub signer: Signer<'info>,

    pub new_authority: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,
}
