use crate::{prelude::*, state::marginfi_account::MarginfiAccount};
use anchor_lang::prelude::*;

pub fn set_account_transfer_authority(
    ctx: Context<MarginfiAccountSetAccountAuthority>,
) -> MarginfiResult {
    // Ensure marginfi_account is dropped out of scope to not exceed stack frame limits
    {
        let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
        let new_account_authority = ctx.accounts.new_authority.key();
        marginfi_account.set_new_account_authority_checked(new_account_authority)?;
    }

    // TODO: add back event (dropped for memory reasons)

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiAccountSetAccountAuthority<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// CHECK: The group is confirmed by the address macro
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
