use crate::{
    events::{AccountEventHeader, MarginfiAccountTransferAccountAuthorityEvent},
    prelude::*,
    state::marginfi_account::MarginfiAccount,
};
use anchor_lang::prelude::*;

pub fn set_account_transfer_authority(
    ctx: Context<MarginfiAccountSetAccountAuthority>,
) -> MarginfiResult {
    let MarginfiAccountSetAccountAuthority {
        marginfi_account: marginfi_account_loader,
        new_authority: new_account_authority,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account_loader.load_mut()?;
    let old_account_authority = marginfi_account.authority;
    marginfi_account.set_new_account_authority_checked(new_account_authority.key())?;

    // assert_ne!(old_account_authority, new_account_authority)?

    emit!(MarginfiAccountTransferAccountAuthorityEvent {
        header: AccountEventHeader {
            signer: Some(new_account_authority.key()),
            marginfi_account: marginfi_account_loader.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        },
        old_account_authority,
        new_account_authority: new_account_authority.key(),
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
