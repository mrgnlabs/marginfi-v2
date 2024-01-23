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
    let marginfi_account_key = ctx.accounts.marginfi_account.key();
    let new_account_authority = ctx.accounts.new_authority.key();
    let signer = ctx.accounts.signer.key();
    let group = ctx.accounts.marginfi_group.key();
    let old_account_authority = marginfi_account.authority;

    marginfi_account.set_new_account_authority_checked(new_account_authority)?;

    emit!(MarginfiAccountTransferAccountAuthorityEvent {
        header: AccountEventHeader {
            signer: Some(signer),
            marginfi_account: marginfi_account_key,
            marginfi_account_authority: new_account_authority,
            marginfi_group: group,
        },
        old_account_authority,
        new_account_authority,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiAccountSetAccountAuthority<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        address = marginfi_account.load()?.group,
    )]
    pub marginfi_group: AccountInfo<'info>,

    #[account(
        address = marginfi_account.load()?.authority,
    )]
    pub signer: Signer<'info>,

    /// CHECK: The new account authority doesn't need explicit checks
    pub new_authority: AccountInfo<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,
}
