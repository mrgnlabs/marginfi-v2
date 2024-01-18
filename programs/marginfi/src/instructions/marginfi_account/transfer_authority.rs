use crate::{
    events::{AccountEventHeader, MarginfiAccountTransferAccountAuthorityEvent},
    prelude::*,
    state::marginfi_account::MarginfiAccount,
};
use anchor_lang::prelude::*;
use solana_program::sysvar::Sysvar;

pub fn set_account_transfer_authority(
    ctx: Context<MarginfiAccountSetAccountAuthority>,
    new_account_authority: Pubkey,
) -> MarginfiResult {
    let MarginfiAccountSetAccountAuthority {
        authority,
        marginfi_group,
        marginfi_account: marginfi_account_loader,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account_loader.load_init()?;

    let old_account_authority = marginfi_account.authority;

    marginfi_account.set_new_account_authority_checked(new_account_authority)?;

    // assert_ne!(old_account_authority, new_account_authority)?

    emit!(MarginfiAccountTransferAccountAuthorityEvent {
        header: AccountEventHeader {
            signer: Some(authority.key()),
            marginfi_account: marginfi_account_loader.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        },
        old_account_authority,
        new_account_authority,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiAccountSetAccountAuthority<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(signer, has_one = authority)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,
}
