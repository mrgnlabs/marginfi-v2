use crate::{
    events::{AccountEventHeader, MarginfiAccountTransferAccountAuthorityEvent},
    prelude::*,
    state::marginfi_account::{LendingAccount, MarginfiAccount, ACCOUNT_DISABLED, ACCOUNT_TRANSFER_AUTHORITY_ALLOWED},
};
use anchor_lang::prelude::*;

pub fn transfer_account_authority(
    ctx: Context<TransferAccountAuthority>,
) -> MarginfiResult {
    anchor_lang::system_program::transfer(ctx.accounts.transfer_fee(), 1000)?;

    let mut old_account = ctx.accounts.old_marginfi_account.load_mut()?;
    check!(
        !old_account.get_flag(ACCOUNT_DISABLED)
            && old_account.get_flag(ACCOUNT_TRANSFER_AUTHORITY_ALLOWED),
        MarginfiError::IllegalAccountAuthorityTransfer
    );

    let mut new_account = ctx.accounts.new_marginfi_account.load_init()?;
    new_account.initialize(old_account.group, ctx.accounts.new_authority.key());
    new_account.lending_account = old_account.lending_account;
    new_account.emissions_destination_account = old_account.emissions_destination_account;
    new_account.account_flags = old_account.account_flags & !ACCOUNT_TRANSFER_AUTHORITY_ALLOWED;
    new_account.migrated_from = ctx.accounts.old_marginfi_account.key();

    old_account.lending_account = LendingAccount::zeroed();
    old_account.unset_flag(ACCOUNT_TRANSFER_AUTHORITY_ALLOWED);
    old_account.set_flag(ACCOUNT_DISABLED);

    emit!(MarginfiAccountTransferAccountAuthorityEvent {
        header: AccountEventHeader {
            signer: Some(ctx.accounts.authority.key()),
            marginfi_account: ctx.accounts.new_marginfi_account.key(),
            marginfi_account_authority: ctx.accounts.new_authority.key(),
            marginfi_group: ctx.accounts.group.key(),
        },
        old_account_authority: ctx.accounts.authority.key(),
        new_account_authority: ctx.accounts.new_authority.key(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct TransferAccountAuthority<'info> {
    #[account(mut, has_one = authority, has_one = group)]
    pub old_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + std::mem::size_of::<MarginfiAccount>()
    )]
    pub new_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub group: AccountLoader<'info, MarginfiGroup>,

    pub authority: Signer<'info>,

    /// CHECK: New authority is unchecked
    pub new_authority: AccountInfo<'info>,

    /// CHECK: Validated against group fee state cache
    #[account(mut)]
    pub global_fee_wallet: AccountInfo<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> TransferAccountAuthority<'info> {
    fn transfer_fee(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, anchor_lang::system_program::Transfer<'info>> {
        CpiContext::new(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: self.fee_payer.to_account_info(),
                to: self.global_fee_wallet.to_account_info(),
            },
        )
    }
}
