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
    #[account(
        mut,
        has_one = authority,
        has_one = group
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// CHECK: Validated against account
    pub group: AccountInfo<'info>,

    pub authority: Signer<'info>,

    /// CHECK: The new account authority doesn't need explicit checks
    pub new_authority: AccountInfo<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,
}
